use crate::memory_bus::MemoryBus;

const GB_WIDTH: usize = 160;
const GB_HEIGHT: usize = 144;

// --- VRAM Debug View Constants ---
// These need to be public if used outside this module
const TILES_PER_ROW_DEBUG: usize = 16; // Keep private if only used here
const NUM_TILES_TO_SHOW: usize = 384; // Keep private if only used here
const VRAM_DEBUG_TILE_HEIGHT: usize = NUM_TILES_TO_SHOW / TILES_PER_ROW_DEBUG; // Keep private
pub const VRAM_DEBUG_WIDTH: usize = TILES_PER_ROW_DEBUG * 8; // Make public
pub const VRAM_DEBUG_HEIGHT: usize = VRAM_DEBUG_TILE_HEIGHT * 8; // Make public

// PPU Timing Constants (in T-cycles)
const DOTS_PER_SCANLINE: u32 = 456;
const SCANLINES_PER_FRAME: u8 = 154; // 144 visible + 10 VBlank

const MODE2_OAM_SCAN_DOTS: u32 = 80;
const MODE3_VRAM_READ_DOTS: u32 = 172;
const MODE0_HBLANK_DOTS: u32 = 204;

// PPU Modes
const HBLANK_MODE: u8 = 0;
const VBLANK_MODE: u8 = 1;
const OAM_SCAN_MODE: u8 = 2;
const VRAM_READ_MODE: u8 = 3;

// Memory Addresses
const VRAM_START: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;
const OAM_START: u16 = 0xFE00;
const OAM_END: u16 = 0xFE9F;

// I/O Register Addresses
const LCDC_ADDR: u16 = 0xFF40;
const STAT_ADDR: u16 = 0xFF41;
const SCY_ADDR: u16 = 0xFF42;
const SCX_ADDR: u16 = 0xFF43;
const LY_ADDR: u16 = 0xFF44;
const LYC_ADDR: u16 = 0xFF45;
const BGP_ADDR: u16 = 0xFF47;
const OBP0_ADDR: u16 = 0xFF48;
const OBP1_ADDR: u16 = 0xFF49;
const WY_ADDR: u16 = 0xFF4A;
const WX_ADDR: u16 = 0xFF4B;

// Interrupt Flags
const IF_ADDR: u16 = 0xFF0F;
const VBLANK_INTERRUPT_BIT: u8 = 0;
const LCD_STAT_INTERRUPT_BIT: u8 = 1;

// LCDC Flags
const LCDC_BG_WIN_ENABLE_PRIORITY: u8 = 0;
const LCDC_OBJ_ENABLE: u8 = 1;
const LCDC_OBJ_SIZE: u8 = 2;
const LCDC_BG_MAP_AREA: u8 = 3;
const LCDC_TILE_DATA_AREA: u8 = 4;
const LCDC_WINDOW_ENABLE: u8 = 5;
const LCDC_WINDOW_MAP_AREA: u8 = 6;
const LCDC_LCD_ENABLE: u8 = 7;

// STAT Flags
const STAT_MODE_FLAG_0: u8 = 0;
const STAT_MODE_FLAG_1: u8 = 1;
const STAT_LYC_EQ_LY_FLAG: u8 = 2;
const STAT_MODE_0_HBLANK_IE: u8 = 3;
const STAT_MODE_1_VBLANK_IE: u8 = 4;
const STAT_MODE_2_OAM_IE: u8 = 5;
const STAT_LYC_EQ_LY_IE: u8 = 6;

pub struct Ppu {
    frame_buffer: [u8; GB_WIDTH * GB_HEIGHT], // Pixel data (0-3 shades) for GB screen
    vram_debug_buffer: [u8; VRAM_DEBUG_WIDTH * VRAM_DEBUG_HEIGHT], // Pixel data for VRAM view
    dots: u32,
    current_scanline: u8,
    lcdc: u8,
    ppu_mode: u8,
    lyc_eq_ly: bool,
    stat_interrupt_line: bool,
    vblank_just_occurred: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            frame_buffer: [0; GB_WIDTH * GB_HEIGHT],
            // Initialize the VRAM debug buffer (e.g., to white/shade 0)
            vram_debug_buffer: [0; VRAM_DEBUG_WIDTH * VRAM_DEBUG_HEIGHT],
            dots: 0,
            current_scanline: 0,
            lcdc: 0x91,
            ppu_mode: HBLANK_MODE,
            lyc_eq_ly: false,
            stat_interrupt_line: false,
            vblank_just_occurred: false,
        }
    }

    /// Get a reference to the current Game Boy screen frame buffer.
    pub fn get_frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    /// Get a reference to the VRAM debug view buffer.
    pub fn get_vram_debug_buffer(&self) -> &[u8] {
        &self.vram_debug_buffer
    }

    /// Steps the PPU by the given number of T-cycles.
    pub fn step(&mut self, cycles: u32, memory_bus: &mut MemoryBus) {
        // Read LCDC at the start, as it dictates PPU operation
        self.lcdc = memory_bus.read_byte(LCDC_ADDR);

        // --- Check if LCD is enabled ---
        if (self.lcdc & (1 << LCDC_LCD_ENABLE)) == 0 {
            // LCD is off - reset state
            if self.dots != 0 || self.current_scanline != 0 || self.ppu_mode != HBLANK_MODE {
                 self.dots = 0;
                 self.current_scanline = 0;
                 self.ppu_mode = HBLANK_MODE;
                 self.lyc_eq_ly = false;
                 self.stat_interrupt_line = false;
                 memory_bus.write_byte(LY_ADDR, 0);
                 let stat_on_bus = memory_bus.read_byte(STAT_ADDR);
                 let stat_to_write = (stat_on_bus & 0b1111_1000) | HBLANK_MODE;
                 memory_bus.write_byte(STAT_ADDR, stat_to_write);

                 // Maybe clear the buffers when LCD turns off? Optional.
                 // self.frame_buffer.fill(0);
                 // self.vram_debug_buffer.fill(0);
            }
            return; // Do nothing else if LCD is off
        }

        self.dots += cycles;

        let mut next_mode = self.ppu_mode;

        match self.ppu_mode {
             OAM_SCAN_MODE => { // Mode 2
                 if self.dots >= MODE2_OAM_SCAN_DOTS {
                     self.dots -= MODE2_OAM_SCAN_DOTS;
                     next_mode = VRAM_READ_MODE;
                 }
             }
             VRAM_READ_MODE => { // Mode 3
                 if self.dots >= MODE3_VRAM_READ_DOTS {
                     self.dots -= MODE3_VRAM_READ_DOTS;
                     next_mode = HBLANK_MODE;
                     // Render the scanline *before* entering HBlank
                     self.render_scanline(memory_bus);
                 }
             }
             HBLANK_MODE => { // Mode 0
                 if self.dots >= DOTS_PER_SCANLINE {
                     self.dots -= DOTS_PER_SCANLINE;
                     self.current_scanline += 1;
                     memory_bus.write_byte(LY_ADDR, self.current_scanline); // Update LY *here* for HBlank->Next Line

                     if self.current_scanline == GB_HEIGHT as u8 { // Transition to VBlank (Line 144)
                         next_mode = VBLANK_MODE;
                         self.vblank_just_occurred = true; // Signal VBlank start for interrupt
                         // --- Consider rendering VRAM debug view once per frame ---
                         // self.render_vram_debug(memory_bus); // Option 1: Do it here
                     } else { // Transition back to OAM Scan for next visible line
                         next_mode = OAM_SCAN_MODE;
                     }
                 }
             }
             VBLANK_MODE => { // Mode 1
                 if self.dots >= DOTS_PER_SCANLINE {
                     self.dots -= DOTS_PER_SCANLINE;
                     self.current_scanline += 1;

                     if self.current_scanline == SCANLINES_PER_FRAME { // End of frame (Line 153 -> 0)
                         self.current_scanline = 0;
                         next_mode = OAM_SCAN_MODE; // Start frame over
                     }
                      // Always update LY during VBlank (lines 144-153 and wrap to 0)
                     memory_bus.write_byte(LY_ADDR, self.current_scanline);
                 }
             }
            _ => unreachable!("Invalid PPU mode"),
        }

        // Update internal PPU mode state *before* checking for interrupts based on the *new* mode
        self.ppu_mode = next_mode;

        // --- Check LYC=LY Coincidence ---
        let lyc = memory_bus.read_byte(LYC_ADDR);
        self.lyc_eq_ly = self.current_scanline == lyc;

        // --- Update STAT Register ---
        let stat_on_bus = memory_bus.read_byte(STAT_ADDR);
        let mut stat_to_write = stat_on_bus & 0b1111_1000; // Preserve IE bits + unused
        stat_to_write |= self.ppu_mode;                   // Set Mode bits
        if self.lyc_eq_ly {
            stat_to_write |= (1 << STAT_LYC_EQ_LY_FLAG);  // Set Coincidence bit
        }
        memory_bus.write_byte(STAT_ADDR, stat_to_write);

        // --- Check for STAT Interrupt Conditions ---
        let mut interrupt_req = false;
        // Use the STAT value *read from the bus* (stat_on_bus) for checking enables
        if self.lyc_eq_ly && (stat_on_bus & (1 << STAT_LYC_EQ_LY_IE)) != 0 {
            interrupt_req = true;
        }
        // Use the *newly set* ppu_mode for checking mode conditions
        if self.ppu_mode == HBLANK_MODE && (stat_on_bus & (1 << STAT_MODE_0_HBLANK_IE)) != 0 {
            interrupt_req = true;
        }
        if self.ppu_mode == VBLANK_MODE && (stat_on_bus & (1 << STAT_MODE_1_VBLANK_IE)) != 0 {
            interrupt_req = true; // Note: VBlank interrupt also has its own vector
        }
        if self.ppu_mode == OAM_SCAN_MODE && (stat_on_bus & (1 << STAT_MODE_2_OAM_IE)) != 0 {
            interrupt_req = true;
        }

        // Check for rising edge of the STAT interrupt line
        if interrupt_req && !self.stat_interrupt_line {
            Self::request_interrupt(memory_bus, LCD_STAT_INTERRUPT_BIT);
        }
        self.stat_interrupt_line = interrupt_req;

        // --- Check for VBlank Interrupt ---
        if self.vblank_just_occurred {
             Self::request_interrupt(memory_bus, VBLANK_INTERRUPT_BIT);
             self.vblank_just_occurred = false;
        }
    }

    /// Renders the contents of VRAM tile data (0x8000-0x97FF) to the debug buffer.
    /// Call this periodically (e.g., once per frame) from your main loop.
    pub fn render_vram_debug(&mut self, memory_bus: &MemoryBus) {
        // Use a simple fixed palette mapping index 0-3 to shades 0-3 for clarity
        let get_debug_shade = |index: u8| index;

        for tile_idx in 0..NUM_TILES_TO_SHOW {
            let tile_addr = VRAM_START + (tile_idx * 16) as u16; // 16 bytes per tile

            // Calculate where this tile goes in the debug buffer grid
            let tile_grid_x = (tile_idx % TILES_PER_ROW_DEBUG);
            let tile_grid_y = (tile_idx / TILES_PER_ROW_DEBUG);
            let base_pixel_x = tile_grid_x * 8;
            let base_pixel_y = tile_grid_y * 8;

            // Render the 8x8 tile
            for y_in_tile in 0..8 {
                let row_addr = tile_addr + (y_in_tile * 2) as u16;
                // Check VRAM bounds just in case, though tile_addr should be correct
                if row_addr >= VRAM_START && row_addr < VRAM_END { // Need row_addr+1 too
                     let byte1 = memory_bus.read_byte(row_addr);
                     let byte2 = memory_bus.read_byte(row_addr + 1);

                     for x_in_tile in 0..8 {
                         // Extract the color index for this pixel
                         // Bit 7 is the leftmost pixel, Bit 0 is the rightmost
                         let bit_pos = 7 - x_in_tile;
                         let bit1 = (byte1 >> bit_pos) & 1;
                         let bit2 = (byte2 >> bit_pos) & 1;
                         let color_index = (bit2 << 1) | bit1;

                         // Get the shade (0-3)
                         let shade = get_debug_shade(color_index);

                         // Calculate the pixel's position in the 1D debug buffer
                         let pixel_x = base_pixel_x + x_in_tile;
                         let pixel_y = base_pixel_y + y_in_tile;
                         let buffer_index = pixel_y * VRAM_DEBUG_WIDTH + pixel_x;

                         // Write to the debug buffer (if within bounds)
                         if buffer_index < self.vram_debug_buffer.len() {
                              self.vram_debug_buffer[buffer_index] = shade;
                         }
                     }
                } else {
                    // Handle case where tile calculation might go out of bounds
                    // Fill the 8x8 area with a default color? Or just skip?
                    for y_fill in 0..8 {
                        for x_fill in 0..8 {
                             let pixel_x = base_pixel_x + x_fill;
                             let pixel_y = base_pixel_y + y_fill;
                             let buffer_index = pixel_y * VRAM_DEBUG_WIDTH + pixel_x;
                             if buffer_index < self.vram_debug_buffer.len() {
                                 self.vram_debug_buffer[buffer_index] = 0; // Fill with white?
                             }
                        }
                    }
                    // Break the inner loop since we can't read this tile row
                    break;
                }
            }
        }
    }


    /// Requests an interrupt by setting the corresponding bit in the IF register.
    fn request_interrupt(memory_bus: &mut MemoryBus, bit: u8) {
        let current_if = memory_bus.read_byte(IF_ADDR);
        memory_bus.write_byte(IF_ADDR, current_if | (1 << bit));
    }

    /// Renders a single scanline to the main frame buffer.
    fn render_scanline(&mut self, memory_bus: &mut MemoryBus) {
         if self.current_scanline >= GB_HEIGHT as u8 {
             return; // Only render visible lines (0-143)
         }
         let y = self.current_scanline;

         // Read necessary registers for rendering this line
         let scy = memory_bus.read_byte(SCY_ADDR);
         let scx = memory_bus.read_byte(SCX_ADDR);
         let wy = memory_bus.read_byte(WY_ADDR);
         let wx = memory_bus.read_byte(WX_ADDR);
         let bgp = memory_bus.read_byte(BGP_ADDR);
         // lcdc flags already read in self.lcdc

         let mut line_buffer = [0u8; GB_WIDTH];

         let window_enabled = (self.lcdc & (1 << LCDC_WINDOW_ENABLE)) != 0;
         let window_visible_y = window_enabled && y >= wy;

         // Determine if the background/window layer itself is enabled (LCDC Bit 0)
         // This affects sprite priority later.
         let bg_win_display_enabled = (self.lcdc & (1 << LCDC_BG_WIN_ENABLE_PRIORITY)) != 0;


         for x in 0..GB_WIDTH as u8 {
             let x_usize = x as usize;

             let mut final_pixel_color = 0; // Default to white
             let mut bg_win_pixel_index = 0; // Store the raw index for priority checks

             // --- Render Background / Window ---
             if bg_win_display_enabled { // Only render BG/Win if master enable is on
                 let window_covers_pixel = window_visible_y && x >= wx.saturating_sub(7);

                 let (tile_map_pixel, _bg_win_tile_prio_attr) = if window_covers_pixel {
                     self.fetch_window_pixel_data(x, y, wx, wy, memory_bus)
                 } else {
                     self.fetch_bg_pixel_data(x, y, scx, scy, memory_bus)
                 };

                 bg_win_pixel_index = tile_map_pixel; // Save the index
                 final_pixel_color = Self::get_color_from_palette(tile_map_pixel, bgp);
             } else {
                 // If BG/Win display is off (LCDC Bit 0 = 0), the whole layer is effectively color 0 (white)
                 bg_win_pixel_index = 0;
                 final_pixel_color = Self::get_color_from_palette(0, bgp); // Render as color 0
             }


             // --- Render Sprites (if enabled) ---
             if (self.lcdc & (1 << LCDC_OBJ_ENABLE)) != 0 {
                 let (sprite_pixel, sprite_oam_prio, sprite_palette) = self.fetch_sprite_pixel_data(x, y, memory_bus);

                 if sprite_pixel != 0 { // Is there a non-transparent sprite pixel here?
                     let sprite_color = Self::get_color_from_palette(sprite_pixel, sprite_palette);

                     // Priority Logic (DMG):
                     // 1. LCDC Bit 0 = 0: Sprites always win.
                     // 2. OAM Priority Flag (Bit 7):
                     //    - 0: Sprite wins over BG/Win (except BG color 0).
                     //    - 1: Sprite loses to BG/Win colors 1, 2, 3.
                     // 3. BG/Win Pixel Color Index: Sprite always wins over BG/Win color 0.

                     let bg_win_is_transparent = bg_win_pixel_index == 0;

                     // Condition: Sprite is drawn if...
                     // - BG/Win display is disabled OR
                     // - Sprite has OAM priority (flag=0) AND BG pixel isn't color 0 OR
                     // - BG/Win pixel IS color 0 (sprite always draws over BG color 0)
                     let sprite_wins_priority = !bg_win_display_enabled || (!sprite_oam_prio) || bg_win_is_transparent;


                     if sprite_wins_priority {
                          final_pixel_color = sprite_color;
                     }
                 }
             }

             line_buffer[x_usize] = final_pixel_color;
         }

         // Copy the rendered line_buffer to the main frame_buffer
         let start_index = y as usize * GB_WIDTH;
         let end_index = start_index + GB_WIDTH;
         if end_index <= self.frame_buffer.len() {
             self.frame_buffer[start_index..end_index].copy_from_slice(&line_buffer);
         }
    }

    /// Helper to get color shade from pixel index and palette register
    #[inline(always)]
    fn get_color_from_palette(pixel_index: u8, palette_reg: u8) -> u8 {
        // Extracts the 2-bit color specified by index (0-3) from the 8-bit palette register
        (palette_reg >> (pixel_index * 2)) & 0b11
    }

    // --- fetch_bg_pixel_data, fetch_window_pixel_data, fetch_sprite_pixel_data remain the same ---
    // (Make sure they handle VRAM reads correctly via MemoryBus)

    // Fetches the raw pixel index (0-3) for the background at screen coordinates (x, y)
    fn fetch_bg_pixel_data(&self, screen_x: u8, screen_y: u8, scx: u8, scy: u8, memory_bus: &MemoryBus) -> (u8, bool) {
        let map_x = screen_x.wrapping_add(scx);
        let map_y = screen_y.wrapping_add(scy);
        let tile_x = (map_x / 8) as u16;
        let tile_y = (map_y / 8) as u16;
        let map_base_addr = if (self.lcdc & (1 << LCDC_BG_MAP_AREA)) == 0 { 0x9800 } else { 0x9C00 };
        let tile_map_offset = tile_y * 32 + tile_x;
        let tile_id_addr = map_base_addr + tile_map_offset;
        let tile_id = memory_bus.read_byte(tile_id_addr);

        let tile_data_base_addr: u16;
        let tile_addr: u16;
        if (self.lcdc & (1 << LCDC_TILE_DATA_AREA)) == 0 {
            tile_data_base_addr = 0x9000;
            let signed_id = tile_id as i8;
            tile_addr = tile_data_base_addr.wrapping_add((signed_id as i16 * 16) as u16);
        } else {
            tile_data_base_addr = 0x8000;
            tile_addr = tile_data_base_addr + (tile_id as u16 * 16);
        }

        let row_in_tile = (map_y % 8) as u16;
        let row_addr = tile_addr + row_in_tile * 2;

        // Basic bounds check for VRAM read
        if row_addr < VRAM_START || row_addr + 1 > VRAM_END { return (0, false); } // Return transparent if out of bounds

        let byte1 = memory_bus.read_byte(row_addr);
        let byte2 = memory_bus.read_byte(row_addr + 1);
        let col_in_tile = 7 - (map_x % 8);
        let bit1 = (byte1 >> col_in_tile) & 1;
        let bit2 = (byte2 >> col_in_tile) & 1;
        let color_index = (bit2 << 1) | bit1;
        let priority = false; // Placeholder DMG
        (color_index, priority)
    }

    // Fetches the raw pixel index (0-3) for the window at screen coordinates (x, y)
    fn fetch_window_pixel_data(&self, screen_x: u8, screen_y: u8, wx:u8, wy:u8, memory_bus: &MemoryBus) -> (u8, bool) {
        // Check if window is actually enabled and visible at this y (already done partially outside)
        if (self.lcdc & (1 << LCDC_WINDOW_ENABLE)) == 0 || screen_y < wy {
             return (0, false); // Should not happen if called correctly, but safe check
        }
        // Check horizontal position (WX-7)
        let effective_wx = wx.saturating_sub(7);
        if screen_x < effective_wx {
            return (0, false); // Pixel is to the left of the window
        }

        let win_x = screen_x - effective_wx;
        let win_y = screen_y - wy;

        let tile_x = (win_x / 8) as u16;
        let tile_y = (win_y / 8) as u16;
        let map_base_addr = if (self.lcdc & (1 << LCDC_WINDOW_MAP_AREA)) == 0 { 0x9800 } else { 0x9C00 };
        let tile_map_offset = tile_y * 32 + tile_x;
        let tile_id_addr = map_base_addr + tile_map_offset;
        let tile_id = memory_bus.read_byte(tile_id_addr);

        let tile_data_base_addr: u16;
        let tile_addr: u16;
        if (self.lcdc & (1 << LCDC_TILE_DATA_AREA)) == 0 {
             tile_data_base_addr = 0x9000;
             let signed_id = tile_id as i8;
             tile_addr = tile_data_base_addr.wrapping_add((signed_id as i16 * 16) as u16);
        } else {
             tile_data_base_addr = 0x8000;
             tile_addr = tile_data_base_addr + (tile_id as u16 * 16);
        }

        let row_in_tile = (win_y % 8) as u16;
        let row_addr = tile_addr + row_in_tile * 2;

        // Basic bounds check for VRAM read
        if row_addr < VRAM_START || row_addr + 1 > VRAM_END { return (0, false); }

        let byte1 = memory_bus.read_byte(row_addr);
        let byte2 = memory_bus.read_byte(row_addr + 1);
        let col_in_tile = 7 - (win_x % 8);
        let bit1 = (byte1 >> col_in_tile) & 1;
        let bit2 = (byte2 >> col_in_tile) & 1;
        let color_index = (bit2 << 1) | bit1;
        let priority = false; // Placeholder DMG
        (color_index, priority)
    }


    // Fetches sprite pixel data
    fn fetch_sprite_pixel_data(&self, screen_x: u8, screen_y: u8, memory_bus: &MemoryBus) -> (u8, bool, u8) {
        // No change needed here, keep as is
        if (self.lcdc & (1 << LCDC_OBJ_ENABLE)) == 0 {
            return (0, false, 0); // Sprites disabled
        }

        let sprite_height = if (self.lcdc & (1 << LCDC_OBJ_SIZE)) != 0 { 16 } else { 8 };
        let mut highest_prio_sprite_pixel = 0;
        let mut highest_prio_sprite_x = u8::MAX; // Use MAX for comparison; lower X wins
        let mut highest_prio_sprite_oam_idx = 40; // Use OAM index as tie-breaker (lower index wins)
        let mut highest_prio_sprite_oam_prio = false;
        let mut highest_prio_sprite_palette = 0;

        let screen_y_u16 = screen_y as u16; // For comparisons involving sprite Y + 16

        // Simplified: Iterate through all 40 sprites. Real PPU uses Mode 2 results.
        for i in 0..40 {
            let oam_addr = OAM_START + (i * 4);
            let sprite_y = memory_bus.read_byte(oam_addr);      // Y pos + 16
            let sprite_x = memory_bus.read_byte(oam_addr + 1);  // X pos + 8
            let tile_index = memory_bus.read_byte(oam_addr + 2);
            let attributes = memory_bus.read_byte(oam_addr + 3);

            // Is sprite on screen vertically? (Coords are top-left + offset)
            // Effective top Y = sprite_y - 16
            let effective_y = sprite_y.wrapping_sub(16);
             if sprite_y != 0 && // sprite_y == 0 means not visible? Check docs. Often true.
                screen_y >= effective_y &&
                screen_y < effective_y.wrapping_add(sprite_height) {

                 // Is sprite on screen horizontally?
                 // Effective left X = sprite_x - 8
                 let effective_x = sprite_x.wrapping_sub(8);
                 if sprite_x != 0 && // sprite_x == 0 means not visible? Check docs. Often true.
                    screen_x >= effective_x &&
                    screen_x < effective_x.wrapping_add(8) { // Sprites are 8 pixels wide

                     // --- Calculate pixel data ---
                     let oam_priority_flag = (attributes >> 7) & 1 == 1; // Bit 7: 1 = BG/Win color 1-3 have priority
                     let y_flip = (attributes >> 6) & 1 == 1;
                     let x_flip = (attributes >> 5) & 1 == 1;
                     let palette_num = (attributes >> 4) & 1; // 0=OBP0, 1=OBP1

                     // Determine row/col within the tile pattern
                     let mut row_in_tile = screen_y - effective_y;
                     if y_flip {
                         row_in_tile = (sprite_height - 1) - row_in_tile;
                     }

                     let mut col_in_tile = screen_x - effective_x;
                      if x_flip {
                          col_in_tile = 7 - col_in_tile;
                      }

                     // Adjust tile index and row for 8x16 sprites
                     let actual_tile_index = if sprite_height == 16 {
                         if row_in_tile < 8 { tile_index & 0xFE } else { tile_index | 0x01 } // Use top/bottom tile
                     } else {
                         tile_index
                     };
                     row_in_tile %= 8; // Row within the 8x8 tile pattern

                     // Sprites always use $8000-$8FFF tile data area
                     let tile_addr = VRAM_START + (actual_tile_index as u16 * 16);
                     let row_addr = tile_addr + (row_in_tile as u16 * 2);

                      // Basic bounds check for VRAM read
                     if row_addr < VRAM_START || row_addr + 1 > VRAM_END { continue; } // Skip if OAM points outside VRAM

                     let byte1 = memory_bus.read_byte(row_addr);
                     let byte2 = memory_bus.read_byte(row_addr + 1);

                     // Extract color index (0-3)
                     let bit1 = (byte1 >> (7 - col_in_tile)) & 1;
                     let bit2 = (byte2 >> (7 - col_in_tile)) & 1;
                     let color_index = (bit2 << 1) | bit1;

                     // --- Check Priority ---
                     if color_index != 0 { // Only consider non-transparent pixels
                         // Priority: Lower X coordinate first, then lower OAM index (i)
                         if sprite_x < highest_prio_sprite_x || (sprite_x == highest_prio_sprite_x && i < highest_prio_sprite_oam_idx) {
                             highest_prio_sprite_pixel = color_index;
                             highest_prio_sprite_x = sprite_x;
                             highest_prio_sprite_oam_idx = i; // Store winning index for tie-breaking
                             highest_prio_sprite_oam_prio = oam_priority_flag;
                             highest_prio_sprite_palette = if palette_num == 0 { memory_bus.read_byte(OBP0_ADDR) } else { memory_bus.read_byte(OBP1_ADDR) };
                         }
                     }
                 }
            }
        }

        (highest_prio_sprite_pixel, highest_prio_sprite_oam_prio, highest_prio_sprite_palette)
    }
}