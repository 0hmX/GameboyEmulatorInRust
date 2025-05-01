use crate::memory_bus::MemoryBus;
use sdl2::pixels::Color; // Keep palette definition for later

const GB_WIDTH: usize = 160;
const GB_HEIGHT: usize = 144;

// PPU Timing Constants (in T-cycles)
const DOTS_PER_SCANLINE: u32 = 456;
const SCANLINES_PER_FRAME: u8 = 154; // 144 visible + 10 VBlank

const MODE2_OAM_SCAN_DOTS: u32 = 80;
const MODE3_VRAM_READ_DOTS: u32 = 172; // Minimum duration, can be longer
const MODE0_HBLANK_DOTS: u32 = 204; // Minimum duration (456 - 80 - 172), can be shorter

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

// Interrupt Flags
const VBLANK_INTERRUPT_BIT: u8 = 0; // Bit 0 in IF/IE
const LCD_STAT_INTERRUPT_BIT: u8 = 1; // Bit 1 in IF/IE

// LCDC Flags (Bit Positions)
const LCDC_BG_WIN_ENABLE_PRIORITY: u8 = 0; // DMG: BG Display Enable; CGB: BG/Win prio
const LCDC_OBJ_ENABLE: u8 = 1;             // Sprite Display Enable
const LCDC_OBJ_SIZE: u8 = 2;               // Sprite Size (0=8x8, 1=8x16)
const LCDC_BG_MAP_AREA: u8 = 3;            // BG Tile Map Display Select (0=9800-9BFF, 1=9C00-9FFF)
const LCDC_TILE_DATA_AREA: u8 = 4;         // BG & Window Tile Data Select (0=8800-97FF, 1=8000-8FFF)
const LCDC_WINDOW_ENABLE: u8 = 5;          // Window Display Enable
const LCDC_WINDOW_MAP_AREA: u8 = 6;        // Window Tile Map Display Select (0=9800-9BFF, 1=9C00-9FFF)
const LCDC_LCD_ENABLE: u8 = 7;             // LCD Display Enable

// STAT Flags (Bit Positions)
const STAT_MODE_FLAG_0: u8 = 0;            // Mode Flag bit 0
const STAT_MODE_FLAG_1: u8 = 1;            // Mode Flag bit 1
const STAT_LYC_EQ_LY_FLAG: u8 = 2;         // Coincidence Flag (Read Only for game)
const STAT_MODE_0_HBLANK_IE: u8 = 3;       // Mode 0 HBlank Interrupt Enable
const STAT_MODE_1_VBLANK_IE: u8 = 4;       // Mode 1 VBlank Interrupt Enable
const STAT_MODE_2_OAM_IE: u8 = 5;          // Mode 2 OAM Interrupt Enable
const STAT_LYC_EQ_LY_IE: u8 = 6;           // LYC=LY Coincidence Interrupt Enable
                                           // Bit 7 is unused

pub struct Ppu {
    frame_buffer: [u8; GB_WIDTH * GB_HEIGHT], // Pixel data (0-3 shades)
    dots: u32,                               // Cycles processed in the current scanline
    current_scanline: u8,                    // Current scanline (LY register) 0-153

    // Internal state mirroring registers (reduces memory bus reads)
    lcdc: u8,
    stat: u8, // We modify STAT internally before writing back
    scy: u8,
    scx: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,

    stat_interrupt_line: bool, // Internal flag for STAT interrupt triggering logic
    vblank_just_occurred: bool, // Flag to ensure VBlank IF is set only once per frame
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            frame_buffer: [0; GB_WIDTH * GB_HEIGHT],
            dots: 0,
            current_scanline: 0,
            lcdc: 0x91, // Default value after boot
            stat: 0x85, // Default value after boot (Mode 1?)
            scy: 0,
            scx: 0,
            lyc: 0,
            bgp: 0xFC,  // Default palette
            obp0: 0xFF, // Default palette
            obp1: 0xFF, // Default palette
            wy: 0,
            wx: 0,
            stat_interrupt_line: false,
            vblank_just_occurred: false,
        }
    }

    /// Get a reference to the current frame buffer.
    pub fn get_frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    /// Steps the PPU by the given number of T-cycles.
    pub fn step(&mut self, cycles: u32, memory_bus: &mut MemoryBus) {
        // Read relevant registers from memory bus first
        // This ensures we work with the latest values CPU might have written
        self.lcdc = memory_bus.read_byte(LCDC_ADDR);
        self.scy = memory_bus.read_byte(SCY_ADDR);
        self.scx = memory_bus.read_byte(SCX_ADDR);
        self.lyc = memory_bus.read_byte(LYC_ADDR);
        self.bgp = memory_bus.read_byte(BGP_ADDR);
        self.obp0 = memory_bus.read_byte(OBP0_ADDR);
        self.obp1 = memory_bus.read_byte(OBP1_ADDR);
        self.wy = memory_bus.read_byte(WY_ADDR);
        self.wx = memory_bus.read_byte(WX_ADDR);
        // Read STAT but preserve the lower 3 read-only bits from our internal state
        let current_stat_in_mem = memory_bus.read_byte(STAT_ADDR);
        self.stat = (current_stat_in_mem & 0b1111_1000) | (self.stat & 0b0000_0111);

        // --- Check if LCD is enabled ---
        if (self.lcdc & (1 << LCDC_LCD_ENABLE)) == 0 {
            // LCD is off, reset PPU state
            if self.dots != 0 || self.current_scanline != 0 {
                self.dots = 0;
                self.current_scanline = 0;
                self.stat &= 0b1111_1100; // Set Mode 0 when LCD disabled? Check PanDocs
                self.stat &= !(1 << STAT_LYC_EQ_LY_FLAG); // Clear coincidence flag
                memory_bus.write_byte(LY_ADDR, 0);
                memory_bus.write_byte(STAT_ADDR, self.stat);
                // Consider clearing the screen buffer here? Or let draw handle it.
            }
            return; // Do nothing else if LCD is off
        }

        self.dots += cycles;

        // Get current mode from internal state
        let current_mode = self.stat & 0b11;
        let mut next_mode = current_mode;

        // --- PPU State Machine ---
        match current_mode {
            OAM_SCAN_MODE => { // Mode 2
                if self.dots >= MODE2_OAM_SCAN_DOTS {
                    self.dots %= MODE2_OAM_SCAN_DOTS; // Keep remainder for next mode
                    next_mode = VRAM_READ_MODE;
                    // TODO: Perform OAM Scan logic here
                }
            }
            VRAM_READ_MODE => { // Mode 3
                // Duration varies, Mode 0 starts when rendering is done for the line.
                // For now, use a fixed minimum split. A real implementation needs pixel FIFO timing.
                if self.dots >= MODE3_VRAM_READ_DOTS {
                    self.dots %= MODE3_VRAM_READ_DOTS; // Keep remainder
                    next_mode = HBLANK_MODE;
                    // This is where the actual rendering for the current scanline happens
                    self.render_scanline(memory_bus);
                }
            }
            HBLANK_MODE => { // Mode 0
                if self.dots >= DOTS_PER_SCANLINE { // End of scanline
                    self.dots %= DOTS_PER_SCANLINE; // Keep remainder
                    self.current_scanline += 1;
                    memory_bus.write_byte(LY_ADDR, self.current_scanline); // Update LY register

                    if self.current_scanline == GB_HEIGHT as u8 { // Transition to VBlank
                        next_mode = VBLANK_MODE;
                        self.vblank_just_occurred = true; // Signal VBlank start
                    } else { // Transition back to OAM Scan for next line
                        next_mode = OAM_SCAN_MODE;
                    }
                }
            }
            VBLANK_MODE => { // Mode 1
                if self.dots >= DOTS_PER_SCANLINE { // End of a VBlank scanline
                    self.dots %= DOTS_PER_SCANLINE; // Keep remainder
                    self.current_scanline += 1;

                    if self.current_scanline == SCANLINES_PER_FRAME { // End of VBlank, wrap around
                        self.current_scanline = 0;
                        next_mode = OAM_SCAN_MODE; // Start frame over
                    }
                    // Always update LY during VBlank
                    memory_bus.write_byte(LY_ADDR, self.current_scanline);
                }
            }
            _ => unreachable!(),
        }

        // Update mode in STAT register state
        self.stat = (self.stat & 0b1111_1100) | next_mode;

        // --- Check LYC=LY Coincidence ---
        let coincidence = self.current_scanline == self.lyc;
        if coincidence {
            self.stat |= (1 << STAT_LYC_EQ_LY_FLAG); // Set coincidence flag
        } else {
            self.stat &= !(1 << STAT_LYC_EQ_LY_FLAG); // Clear coincidence flag
        }

        // --- Check for STAT Interrupt Conditions ---
        // Logic based on gbdev wiki diagrams: Interrupt triggers on the rising edge
        // of the internal STAT interrupt line.
        let mut interrupt_req = false;
        if coincidence && (self.stat & (1 << STAT_LYC_EQ_LY_IE)) != 0 {
             interrupt_req = true; // LYC=LY interrupt enabled and condition met
        }
        if next_mode == HBLANK_MODE && (self.stat & (1 << STAT_MODE_0_HBLANK_IE)) != 0 {
            interrupt_req = true; // Mode 0 interrupt enabled and condition met
        }
         if next_mode == VBLANK_MODE && (self.stat & (1 << STAT_MODE_1_VBLANK_IE)) != 0 {
             interrupt_req = true; // Mode 1 interrupt enabled and condition met
         }
        if next_mode == OAM_SCAN_MODE && (self.stat & (1 << STAT_MODE_2_OAM_IE)) != 0 {
            interrupt_req = true; // Mode 2 interrupt enabled and condition met
        }

        // Check for rising edge of the STAT interrupt line
        if interrupt_req && !self.stat_interrupt_line {
            // Request LCD STAT Interrupt
             Self::request_interrupt(memory_bus, LCD_STAT_INTERRUPT_BIT);
        }
        self.stat_interrupt_line = interrupt_req;


        // --- Check for VBlank Interrupt ---
        if self.vblank_just_occurred {
             Self::request_interrupt(memory_bus, VBLANK_INTERRUPT_BIT);
             self.vblank_just_occurred = false; // Only request once per frame
        }


        // Write the possibly updated STAT register back to memory
        // Only write enabled bits + mode flags + coincidence flag
        let current_stat_on_bus = memory_bus.read_byte(STAT_ADDR);
        let stat_to_write = (self.stat & 0b0111_1111) | (current_stat_on_bus & 0b1000_0000); // Keep bit 7 from bus
        memory_bus.write_byte(STAT_ADDR, stat_to_write);
    }

    /// Requests an interrupt by setting the corresponding bit in the IF register.
    fn request_interrupt(memory_bus: &mut MemoryBus, bit: u8) {
        let current_if = memory_bus.read_byte(0xFF0F);
        memory_bus.write_byte(0xFF0F, current_if | (1 << bit));
    }

    /// Renders a single scanline to the frame buffer.
    /// This is a placeholder and needs full implementation.
    fn render_scanline(&mut self, _memory_bus: &MemoryBus) {
         if self.current_scanline >= GB_HEIGHT as u8 {
             return; // Only render visible lines
         }
         let y = self.current_scanline as usize;

         // *** PLACEHOLDER RENDERING ***
         // Replace this with actual Background, Window, and Sprite rendering logic

         // Example: Fill scanline based on current mode for visualization
         let mode = self.stat & 0b11;
         let shade = mode; // Use mode 0-3 as the shade

         // Example: Fill based on LY
         // let shade = (self.current_scanline / (GB_HEIGHT as u8 / 4)) % 4;

         for x in 0..GB_WIDTH {
             let index = y * GB_WIDTH + x;
             self.frame_buffer[index] = shade;
         }

         // *** END PLACEHOLDER ***

         // --- Real Rendering Logic Outline ---
         for x in 0..GB_WIDTH {
             let index = y * GB_WIDTH + x;

             // 1. Calculate Background pixel color & priority
             let (bg_color, bg_prio) = self.fetch_bg_pixel(x, _memory_bus);

             // 2. Calculate Window pixel color & priority (if active)
             let (win_color, win_prio) = self.fetch_window_pixel(x, _memory_bus);

             // 3. Calculate Sprite pixel color & priority
             // This involves checking sprites scanned during Mode 2
             let (spr_color, spr_prio_flag, spr_has_priority_over_bg) = self.fetch_sprite_pixel(x, _memory_bus);

             // 4. Determine final pixel based on priority rules and LCDC flags
             let mut final_color = bg_color; // Default to BG

             // Apply window if enabled and covers pixel
             if self.is_window_pixel(x, (y as u8)) {
                 final_color = win_color;
                 // Window always has priority over BG? Check details.
             }

             // Apply sprite if enabled, visible, and has priority
             if (self.lcdc & (1 << LCDC_OBJ_ENABLE)) != 0 && spr_color != 0 { // Sprite pixel is not transparent
                 let bg_is_transparent = final_color == 0; // Check if BG/Win pixel is color 0

                 // Sprite priority conditions (Refer to PanDocs or Ultimate Game Boy talk)
                 // - Sprite wins if LCDC BG/Win Enable/Priority (Bit 0) is 0
                 // - Sprite wins if BG/Win pixel is transparent (color 0)
                 // - Sprite wins if Sprite's OAM priority flag is 0 (OBJ-to-BG Priority)
                 let sprite_wins = ((self.lcdc & (1<<LCDC_BG_WIN_ENABLE_PRIORITY)) == 0)
                                  || bg_is_transparent
                                  || spr_has_priority_over_bg;

                 if sprite_wins {
                     final_color = spr_color;
                 }
             }

             self.frame_buffer[index] = final_color;
         }
    }

    // --- Placeholder rendering helpers (To be fully implemented) ---

    fn fetch_bg_pixel(&self, x: usize, memory_bus: &MemoryBus) -> (u8, bool) {
        // TODO: Implement BG pixel fetching based on LCDC, SCX, SCY, BGP, VRAM tile data/map
        (0, false) // Return color 0, priority false (placeholder)
    }

    fn fetch_window_pixel(&self, x: usize, memory_bus: &MemoryBus) -> (u8, bool) {
         // TODO: Implement Window pixel fetching if window enabled and visible at x,y
        (0, false) // Return color 0, priority false (placeholder)
    }

    fn is_window_pixel(&self, x:usize, y:u8) -> bool {
         // TODO: Check LCDC.WINDOW_ENABLE and if x, y are within window bounds (wx, wy)
         false
    }

    fn fetch_sprite_pixel(&self, x: usize, memory_bus: &MemoryBus) -> (u8, bool, bool) {
         // TODO: Implement Sprite pixel fetching by checking sprites found in OAM scan
         // Needs to handle priority, palettes (OBP0/1), flipping, size (8x8/8x16)
        (0, false, false) // Return color 0, OAM prio flag, Sprite-wins-over-BG flag (placeholders)
    }
}