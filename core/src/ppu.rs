use crate::memory_bus::MemoryBus;
// Palette definition removed as it's not used directly in PPU logic itself
// use sdl2::pixels::Color;

const GB_WIDTH: usize = 160;
const GB_HEIGHT: usize = 144;

// PPU Timing Constants (in T-cycles)
const DOTS_PER_SCANLINE: u32 = 456;
const SCANLINES_PER_FRAME: u8 = 154; // 144 visible + 10 VBlank

const MODE2_OAM_SCAN_DOTS: u32 = 80;
// NOTE: Mode 3 duration varies in real hardware based on pixel pipeline state.
// Using a fixed minimum duration is a common simplification.
const MODE3_VRAM_READ_DOTS: u32 = 172;
const MODE0_HBLANK_DOTS: u32 = 204; // Usually 456 - MODE2 - MODE3_ACTUAL

// PPU Modes
const HBLANK_MODE: u8 = 0;
const VBLANK_MODE: u8 = 1;
const OAM_SCAN_MODE: u8 = 2;
const VRAM_READ_MODE: u8 = 3;

// Memory Addresses (Correct)
const VRAM_START: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;
const OAM_START: u16 = 0xFE00;
const OAM_END: u16 = 0xFE9F;

// I/O Register Addresses (Correct)
const LCDC_ADDR: u16 = 0xFF40; // LCD Control
const STAT_ADDR: u16 = 0xFF41; // LCD Status
const SCY_ADDR: u16 = 0xFF42;  // Scroll Y
const SCX_ADDR: u16 = 0xFF43;  // Scroll X
const LY_ADDR: u16 = 0xFF44;   // LCD Y Coordinate (Current Scanline)
const LYC_ADDR: u16 = 0xFF45;  // LY Compare
const BGP_ADDR: u16 = 0xFF47;  // BG Palette Data
const OBP0_ADDR: u16 = 0xFF48; // Object Palette 0 Data
const OBP1_ADDR: u16 = 0xFF49; // Object Palette 1 Data
const WY_ADDR: u16 = 0xFF4A;   // Window Y Position
const WX_ADDR: u16 = 0xFF4B;   // Window X Position

// Interrupt Flags (Correct)
const IF_ADDR: u16 = 0xFF0F; // Interrupt Flag Register
const VBLANK_INTERRUPT_BIT: u8 = 0; // Bit 0 in IF/IE
const LCD_STAT_INTERRUPT_BIT: u8 = 1; // Bit 1 in IF/IE

// LCDC Flags (Bit Positions) (Correct)
const LCDC_BG_WIN_ENABLE_PRIORITY: u8 = 0;
const LCDC_OBJ_ENABLE: u8 = 1;
const LCDC_OBJ_SIZE: u8 = 2;
const LCDC_BG_MAP_AREA: u8 = 3;
const LCDC_TILE_DATA_AREA: u8 = 4;
const LCDC_WINDOW_ENABLE: u8 = 5;
const LCDC_WINDOW_MAP_AREA: u8 = 6;
const LCDC_LCD_ENABLE: u8 = 7;

// STAT Flags (Bit Positions) (Correct)
const STAT_MODE_FLAG_0: u8 = 0;
const STAT_MODE_FLAG_1: u8 = 1;
const STAT_LYC_EQ_LY_FLAG: u8 = 2;         // Read Only (by PPU, CPU can write?)
const STAT_MODE_0_HBLANK_IE: u8 = 3;
const STAT_MODE_1_VBLANK_IE: u8 = 4;
const STAT_MODE_2_OAM_IE: u8 = 5;
const STAT_LYC_EQ_LY_IE: u8 = 6;
// Bit 7 is unused (reads as 1?)

pub struct Ppu {
    frame_buffer: [u8; GB_WIDTH * GB_HEIGHT], // Pixel data (0-3 shades)
    dots: u32,                               // Cycles processed in the current scanline
    current_scanline: u8,                    // Current scanline (LY register) 0-153

    // Internal state partly mirroring registers, but not always 1:1
    lcdc: u8,
    // Only store the state bits the PPU *controls* internally (Mode, Coincidence Flag)
    // Interrupt enables are read directly from the bus when needed.
    ppu_mode: u8,
    lyc_eq_ly: bool,
    // Palettes and scroll registers are read when needed during rendering
    // scy: u8,
    // scx: u8,
    // lyc: u8, read from bus
    // bgp: u8, read from bus
    // obp0: u8, read from bus
    // obp1: u8, read from bus
    // wy: u8, read from bus
    // wx: u8, read from bus

    stat_interrupt_line: bool, // Internal flag for STAT interrupt triggering logic
    vblank_just_occurred: bool, // Flag to ensure VBlank IF is set only once per frame
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            frame_buffer: [0; GB_WIDTH * GB_HEIGHT],
            dots: 0,
            current_scanline: 0,
            lcdc: 0x91, // Default value after boot, read from bus on step
            ppu_mode: HBLANK_MODE, // Assume starting in Mode 0 (or perhaps OAM_SCAN_MODE?) Check boot sequence if not skipping.
            lyc_eq_ly: false,
            stat_interrupt_line: false,
            vblank_just_occurred: false,
            // Other fields like palettes, scroll regs will be read from bus on demand
        }
    }

    /// Get a reference to the current frame buffer.
    pub fn get_frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    /// Steps the PPU by the given number of T-cycles.
    pub fn step(&mut self, cycles: u32, memory_bus: &mut MemoryBus) {
        // Read LCDC at the start, as it dictates PPU operation
        self.lcdc = memory_bus.read_byte(LCDC_ADDR);

        // --- Check if LCD is enabled ---
        if (self.lcdc & (1 << LCDC_LCD_ENABLE)) == 0 {
            // LCD is off
            // Reset PPU state if it wasn't already off
            if self.dots != 0 || self.current_scanline != 0 || self.ppu_mode != HBLANK_MODE {
                self.dots = 0;
                self.current_scanline = 0;
                self.ppu_mode = HBLANK_MODE; // Set to Mode 0 when disabled
                self.lyc_eq_ly = false;      // Coincidence cannot happen
                self.stat_interrupt_line = false; // Reset interrupt line state

                memory_bus.write_byte(LY_ADDR, 0);

                // Update STAT register: Preserve bits 3-7, set Mode 0, clear coincidence
                let stat_on_bus = memory_bus.read_byte(STAT_ADDR);
                let stat_to_write = (stat_on_bus & 0b1111_1000) | HBLANK_MODE; // Mode 0, coincidence clear implicitly
                memory_bus.write_byte(STAT_ADDR, stat_to_write);
            }
            return; // Do nothing else if LCD is off
        }

        self.dots += cycles;

        // Determine next state based on current mode and dots accumulated
        let mut next_mode = self.ppu_mode;

        match self.ppu_mode {
            OAM_SCAN_MODE => { // Mode 2
                if self.dots >= MODE2_OAM_SCAN_DOTS {
                    self.dots -= MODE2_OAM_SCAN_DOTS; // Subtract threshold
                    next_mode = VRAM_READ_MODE;
                    // OAM scan logic would happen here in a real implementation
                }
            }
            VRAM_READ_MODE => { // Mode 3
                // Simplified: Assume transition happens after fixed duration.
                // Real PPU transitions when pixel processing for the line is done.
                if self.dots >= MODE3_VRAM_READ_DOTS {
                    self.dots -= MODE3_VRAM_READ_DOTS; // Subtract threshold
                    next_mode = HBLANK_MODE;
                    // Render the scanline just before transitioning to HBlank
                    self.render_scanline(memory_bus);
                }
            }
            HBLANK_MODE => { // Mode 0
                if self.dots >= DOTS_PER_SCANLINE { // End of scanline
                    self.dots -= DOTS_PER_SCANLINE; // Subtract threshold (start next line)
                    self.current_scanline += 1;

                    if self.current_scanline == GB_HEIGHT as u8 { // Transition to VBlank (Line 144)
                        next_mode = VBLANK_MODE;
                        self.vblank_just_occurred = true; // Signal VBlank start for interrupt
                    } else { // Transition back to OAM Scan for next line
                        next_mode = OAM_SCAN_MODE;
                    }
                    // Update LY register *after* incrementing
                    memory_bus.write_byte(LY_ADDR, self.current_scanline);
                }
            }
            VBLANK_MODE => { // Mode 1
                if self.dots >= DOTS_PER_SCANLINE { // End of a VBlank scanline
                    self.dots -= DOTS_PER_SCANLINE; // Subtract threshold
                    self.current_scanline += 1;

                    if self.current_scanline == SCANLINES_PER_FRAME { // End of frame (Line 154 wraps to 0)
                        self.current_scanline = 0;
                        next_mode = OAM_SCAN_MODE; // Start frame over
                    }
                    // Always update LY during VBlank
                    memory_bus.write_byte(LY_ADDR, self.current_scanline);
                }
            }
            _ => unreachable!("Invalid PPU mode"),
        }

        // Update internal PPU mode state *before* checking for interrupts based on the *new* mode
        self.ppu_mode = next_mode;

        // --- Check LYC=LY Coincidence ---
        // Read LYC dynamically as CPU can change it anytime
        let lyc = memory_bus.read_byte(LYC_ADDR);
        self.lyc_eq_ly = self.current_scanline == lyc;

        // --- Update STAT Register ---
        // Read the *current* value from the bus to get CPU's latest IE bits
        let stat_on_bus = memory_bus.read_byte(STAT_ADDR);
        let mut stat_to_write = stat_on_bus & 0b1111_1000; // Preserve bits 3-7 (IE flags + unused)
        stat_to_write |= self.ppu_mode;                   // Set Mode bits (0-1)
        if self.lyc_eq_ly {
            stat_to_write |= (1 << STAT_LYC_EQ_LY_FLAG);  // Set Coincidence bit (2)
        }
        // Write the updated STAT back immediately
        memory_bus.write_byte(STAT_ADDR, stat_to_write);


        // --- Check for STAT Interrupt Conditions ---
        // An interrupt is requested if *any* enabled condition is met by the current state.
        let mut interrupt_req = false;
        if self.lyc_eq_ly && (stat_on_bus & (1 << STAT_LYC_EQ_LY_IE)) != 0 {
            interrupt_req = true; // LYC=LY interrupt enabled and condition met
        }
        // Mode-based checks: Use the *newly set* ppu_mode
        if self.ppu_mode == HBLANK_MODE && (stat_on_bus & (1 << STAT_MODE_0_HBLANK_IE)) != 0 {
            interrupt_req = true; // Mode 0 interrupt enabled and condition met
        }
        if self.ppu_mode == VBLANK_MODE && (stat_on_bus & (1 << STAT_MODE_1_VBLANK_IE)) != 0 {
            interrupt_req = true; // Mode 1 interrupt enabled and condition met
        }
        if self.ppu_mode == OAM_SCAN_MODE && (stat_on_bus & (1 << STAT_MODE_2_OAM_IE)) != 0 {
            interrupt_req = true; // Mode 2 interrupt enabled and condition met
        }

        // Check for rising edge of the STAT interrupt line
        // This prevents spamming interrupts if a condition remains true over multiple steps
        if interrupt_req && !self.stat_interrupt_line {
            Self::request_interrupt(memory_bus, LCD_STAT_INTERRUPT_BIT);
        }
        // Update the internal line state for the next step's check
        self.stat_interrupt_line = interrupt_req;


        // --- Check for VBlank Interrupt ---
        if self.vblank_just_occurred {
             Self::request_interrupt(memory_bus, VBLANK_INTERRUPT_BIT);
             self.vblank_just_occurred = false; // Reset flag after requesting
        }
    }

    /// Requests an interrupt by setting the corresponding bit in the IF register.
    fn request_interrupt(memory_bus: &mut MemoryBus, bit: u8) {
        let current_if = memory_bus.read_byte(IF_ADDR);
        memory_bus.write_byte(IF_ADDR, current_if | (1 << bit));
    }

    /// Renders a single scanline to the frame buffer.
    /// Needs full implementation based on VRAM data, palettes, LCDC flags, etc.
    fn render_scanline(&mut self, memory_bus: &mut MemoryBus) {
         if self.current_scanline >= GB_HEIGHT as u8 {
             return; // Only render visible lines (0-143)
         }
         let y = self.current_scanline; // Use u8 directly

         // Read necessary registers for rendering this line
         let scy = memory_bus.read_byte(SCY_ADDR);
         let scx = memory_bus.read_byte(SCX_ADDR);
         let wy = memory_bus.read_byte(WY_ADDR);
         let wx = memory_bus.read_byte(WX_ADDR);
         let bgp = memory_bus.read_byte(BGP_ADDR);
         // obj palettes needed for sprites
         // lcdc flags already read in self.lcdc

         // *** Clear placeholder rendering ***
         // The actual rendering logic below will fill the buffer.
         // Remove the old mode/LY based filling.

         // --- Real Rendering Logic ---
         let mut line_buffer = [0u8; GB_WIDTH]; // Temp buffer for the line being rendered

         // Determine if window is active for this scanline
         let window_enabled = (self.lcdc & (1 << LCDC_WINDOW_ENABLE)) != 0;
         let window_visible_y = window_enabled && y >= wy;

         for x in 0..GB_WIDTH as u8 { // Iterate 0-159
             let x_usize = x as usize;

             // Determine if the window covers this pixel
             let window_covers_pixel = window_visible_y && x >= wx.saturating_sub(7); // WX is weird (wx-7)

             // Fetch BG/Win pixel color (Choose based on window state)
             let (tile_map_pixel, bg_win_tile_prio_attr) = if window_covers_pixel { // Fetch Window Pixel
                 self.fetch_window_pixel_data(x, y, wx, wy, memory_bus)
             } else { // Fetch BG Pixel
                 self.fetch_bg_pixel_data(x, y, scx, scy, memory_bus)
             };

             // Translate tile map pixel index (0-3) to actual color using BGP
             let mut final_pixel_color = Self::get_color_from_palette(tile_map_pixel, bgp);

             // Fetch Sprite pixel color and attributes for this x coordinate
             // This is complex: Needs OAM scan results, checks multiple sprites
             let (sprite_pixel, sprite_oam_prio, sprite_palette) = self.fetch_sprite_pixel_data(x, y, memory_bus);

             // Combine based on priority rules if a non-transparent sprite pixel exists
             if sprite_pixel != 0 { // Is there a sprite here?
                 let sprite_color = Self::get_color_from_palette(sprite_pixel, sprite_palette);

                 let bg_win_is_transparent = tile_map_pixel == 0; // Check underlying BG/Win pixel

                 // Determine if sprite wins priority (Simplified - Check PanDocs for CGB specifics if needed)
                 // DMG Priority:
                 // 1. OAM Priority flag (0=Sprite above BG, 1=Sprite behind BG colors 1-3)
                 // 2. BG Color 0 Transparency (Sprite always draws over BG color 0)
                 // 3. LCDC Bit 0 (BG Display) (0=Sprites always win, 1=Normal priority) - Less relevant now? Check interpretation
                 let sprite_has_priority = sprite_oam_prio // OAM Flag: 0=Sprite Wins Prio
                                            || bg_win_is_transparent; // Sprite wins over BG color 0

                 // LCDC Bit 0 is more complex, often called BG/Window Enable/Priority
                 // If LCDC Bit 0 is 0, BG/Window basically lose priority.
                 let bg_enabled = (self.lcdc & (1 << LCDC_BG_WIN_ENABLE_PRIORITY)) != 0;


                // Combine conditions: Sprite should be drawn if...
                if !bg_enabled || sprite_has_priority {
                     final_pixel_color = sprite_color;
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
        // pixel_index is 0, 1, 2, or 3
        // palette_reg maps these indices to shades 0-3
        match pixel_index {
            0 => (palette_reg >> 0) & 0b11,
            1 => (palette_reg >> 2) & 0b11,
            2 => (palette_reg >> 4) & 0b11,
            3 => (palette_reg >> 6) & 0b11,
            _ => 0, // Should not happen
        }
    }

    // --- Detailed rendering helpers (To be fully implemented) ---

    // Fetches the raw pixel index (0-3) for the background at screen coordinates (x, y)
    fn fetch_bg_pixel_data(&self, screen_x: u8, screen_y: u8, scx: u8, scy: u8, memory_bus: &MemoryBus) -> (u8, bool) {
        // 1. Calculate absolute pixel coordinates in the full background map (256x256)
        let map_x = screen_x.wrapping_add(scx);
        let map_y = screen_y.wrapping_add(scy);

        // 2. Determine which tile in the map this pixel belongs to (8x8 tiles)
        let tile_x = (map_x / 8) as u16;
        let tile_y = (map_y / 8) as u16;

        // 3. Determine the base address of the BG tile map (LCDC Bit 3)
        let map_base_addr = if (self.lcdc & (1 << LCDC_BG_MAP_AREA)) == 0 { 0x9800 } else { 0x9C00 };
        let tile_map_offset = tile_y * 32 + tile_x; // 32 tiles per row in map
        let tile_id_addr = map_base_addr + tile_map_offset;

        // 4. Read the tile ID from the map
        let tile_id = memory_bus.read_byte(tile_id_addr);

        // 5. Determine the base address of the tile data (LCDC Bit 4)
        let tile_data_base_addr: u16;
        let tile_addr: u16;

        if (self.lcdc & (1 << LCDC_TILE_DATA_AREA)) == 0 {
            // Use $8800-$97FF range (signed tile IDs)
            tile_data_base_addr = 0x9000; // Center point
            let signed_id = tile_id as i8;
            tile_addr = tile_data_base_addr.wrapping_add((signed_id as i16 * 16) as u16); // 16 bytes per tile
        } else {
            // Use $8000-$8FFF range (unsigned tile IDs)
            tile_data_base_addr = 0x8000;
            tile_addr = tile_data_base_addr + (tile_id as u16 * 16); // 16 bytes per tile
        }

        // 6. Determine the specific row within the tile (2 bytes per row)
        let row_in_tile = (map_y % 8) as u16;
        let row_addr = tile_addr + row_in_tile * 2;

        // 7. Read the two bytes for the tile row
        let byte1 = memory_bus.read_byte(row_addr);
        let byte2 = memory_bus.read_byte(row_addr + 1);

        // 8. Determine the specific column within the tile row (0-7)
        let col_in_tile = 7 - (map_x % 8); // Pixels stored high bit first

        // 9. Extract the 2 bits for the pixel color index
        let bit1 = (byte1 >> col_in_tile) & 1;
        let bit2 = (byte2 >> col_in_tile) & 1;
        let color_index = (bit2 << 1) | bit1;

        // TODO: Handle CGB tile attributes (priority) if implementing CGB
        let priority = false; // Placeholder for DMG

        (color_index, priority)
    }

     // Fetches the raw pixel index (0-3) for the window at screen coordinates (x, y)
    fn fetch_window_pixel_data(&self, screen_x: u8, screen_y: u8, wx:u8, wy:u8, memory_bus: &MemoryBus) -> (u8, bool) {
        // 1. Calculate coordinates relative to window top-left (WX-7, WY)
        // Need to handle WX-7 carefully if WX < 7
        let win_x = screen_x.saturating_sub(wx.saturating_sub(7));
        let win_y = screen_y - wy; // Assumes window is visible (checked before call)

        // 2. Determine which tile in the map this pixel belongs to
        let tile_x = (win_x / 8) as u16;
        let tile_y = (win_y / 8) as u16;

        // 3. Determine the base address of the Window tile map (LCDC Bit 6)
        let map_base_addr = if (self.lcdc & (1 << LCDC_WINDOW_MAP_AREA)) == 0 { 0x9800 } else { 0x9C00 };
        let tile_map_offset = tile_y * 32 + tile_x;
        let tile_id_addr = map_base_addr + tile_map_offset;

        // 4. Read the tile ID
        let tile_id = memory_bus.read_byte(tile_id_addr);

        // 5. Determine the base address of the tile data (LCDC Bit 4 - same as BG)
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

        // 6. Determine the specific row within the tile
        let row_in_tile = (win_y % 8) as u16;
        let row_addr = tile_addr + row_in_tile * 2;

        // 7. Read the two bytes for the tile row
        let byte1 = memory_bus.read_byte(row_addr);
        let byte2 = memory_bus.read_byte(row_addr + 1);

        // 8. Determine the specific column within the tile row
        let col_in_tile = 7 - (win_x % 8);

        // 9. Extract the 2 bits for the pixel color index
        let bit1 = (byte1 >> col_in_tile) & 1;
        let bit2 = (byte2 >> col_in_tile) & 1;
        let color_index = (bit2 << 1) | bit1;

        // TODO: Handle CGB tile attributes if implementing CGB
        let priority = false; // Placeholder for DMG

        (color_index, priority)
    }

    // Fetches sprite pixel data (color index 0-3, OAM priority flag, palette register value)
    // This is complex, needs OAM scan results. Returns the topmost, highest-priority sprite pixel.
    fn fetch_sprite_pixel_data(&self, screen_x: u8, screen_y: u8, memory_bus: &MemoryBus) -> (u8, bool, u8) {
        if (self.lcdc & (1 << LCDC_OBJ_ENABLE)) == 0 {
            return (0, false, 0); // Sprites disabled
        }

        let sprite_height = if (self.lcdc & (1 << LCDC_OBJ_SIZE)) != 0 { 16 } else { 8 };
        let mut highest_prio_sprite_pixel = 0; // Color index (0 is transparent)
        let mut highest_prio_sprite_x = 255; // X coord of highest prio sprite (lower X wins tie)
        let mut highest_prio_sprite_oam_prio = false; // OAM prio flag (0=win, 1=lose)
        let mut highest_prio_sprite_palette = 0;      // OBP0 or OBP1

        // Iterate through OAM (0xFE00-0xFE9F) - Max 40 sprites, 4 bytes each
        // In reality, Mode 2 finds the first 10 sprites overlapping this scanline.
        // This simplified version checks all 40.
        for i in 0..40 {
            let oam_addr = OAM_START + (i * 4);
            let sprite_y = memory_bus.read_byte(oam_addr); // Y position + 16
            let sprite_x = memory_bus.read_byte(oam_addr + 1); // X position + 8
            let tile_index = memory_bus.read_byte(oam_addr + 2);
            let attributes = memory_bus.read_byte(oam_addr + 3);

            // Check if sprite is visible on this scanline
            let screen_y_u16 = screen_y as u16; // For comparison
            if sprite_y != 0 && screen_y_u16 >= (sprite_y.wrapping_sub(16) as u16) && screen_y_u16 < (sprite_y.wrapping_sub(16).wrapping_add(sprite_height as u8) as u16) {
                // Sprite overlaps vertically

                // Check if sprite is visible horizontally (partially or fully)
                 if sprite_x != 0 && screen_x >= sprite_x.wrapping_sub(8) && screen_x < sprite_x {
                     // Sprite overlaps horizontally

                     // --- Calculate pixel data ---
                     let oam_priority = (attributes >> 7) & 1 == 1; // Bit 7: BG and Window over OBJ (0=OBJ above BG, 1=OBJ behind BG colors 1-3)
                     let y_flip = (attributes >> 6) & 1 == 1;
                     let x_flip = (attributes >> 5) & 1 == 1;
                     let palette_num = (attributes >> 4) & 1; // 0=OBP0, 1=OBP1

                     // Determine which row of the tile to fetch
                     let mut row_in_tile = screen_y_u16.wrapping_sub((sprite_y.wrapping_sub(16) as u16));
                     if y_flip {
                         row_in_tile = (sprite_height as u16 - 1) - row_in_tile;
                     }

                     // Handle 8x16 sprites (tile index ignores bit 0)
                     let actual_tile_index = if sprite_height == 16 {
                         if row_in_tile < 8 { tile_index & 0xFE } else { tile_index | 0x01 }
                     } else {
                         tile_index
                     };
                     row_in_tile %= 8; // Use row within the 8x8 part

                     let tile_addr = 0x8000 + (actual_tile_index as u16 * 16); // Sprites always use $8000 data area
                     let row_addr = tile_addr + row_in_tile * 2;

                     let byte1 = memory_bus.read_byte(row_addr);
                     let byte2 = memory_bus.read_byte(row_addr + 1);

                     // Determine column within tile
                     let mut col_in_tile = screen_x.wrapping_sub(sprite_x.wrapping_sub(8));
                     if x_flip {
                         col_in_tile = 7 - col_in_tile;
                     }

                     // Extract color index
                     let bit1 = (byte1 >> (7 - col_in_tile)) & 1;
                     let bit2 = (byte2 >> (7 - col_in_tile)) & 1;
                     let color_index = (bit2 << 1) | bit1;

                     // --- Check Priority ---
                     if color_index != 0 { // Only consider non-transparent pixels
                         // Priority: Lower X coordinate wins. OAM index is tie-breaker (implicit via loop order).
                         if sprite_x < highest_prio_sprite_x {
                             highest_prio_sprite_pixel = color_index;
                             highest_prio_sprite_x = sprite_x;
                             highest_prio_sprite_oam_prio = oam_priority;
                             highest_prio_sprite_palette = if palette_num == 0 { memory_bus.read_byte(OBP0_ADDR) } else { memory_bus.read_byte(OBP1_ADDR) };
                         }
                     }
                 }
            }
        }

        (highest_prio_sprite_pixel, highest_prio_sprite_oam_prio, highest_prio_sprite_palette)
    }
}