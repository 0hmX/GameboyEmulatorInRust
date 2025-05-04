use super::constants::*;

/// Holds the internal state of the PPU, primarily related to timing and modes.
#[derive(Debug, Clone)]
pub struct PpuState {
    pub(super) dots: u32, // Current dot within the scanline (T-cycle counter)
    pub(super) current_scanline: u8, // Current scanline (LY register value, 0-153)
    pub(super) ppu_mode: u8, // Current PPU mode (0, 1, 2, 3)
    pub(super) lyc_eq_ly: bool, // Status of LYC == LY comparison
    pub(super) stat_interrupt_line: bool, // Tracks the state of the STAT interrupt line (high/low)
    pub(super) vblank_just_occurred: bool, // Flag to signal VBlank interrupt on mode transition
    pub(super) lcdc: u8,  // Cache of LCDC register value for the current step
    pub(super) stat: u8,  // Cache of STAT register value for the current step
}

impl PpuState {
    pub fn new() -> Self {
        PpuState {
            dots: 0,
            current_scanline: 0,
            ppu_mode: OAM_SCAN_MODE, // Start in OAM scan? Or Mode 0? Check boot sequence. Let's assume OAM.
            lyc_eq_ly: false,
            stat_interrupt_line: false,
            vblank_just_occurred: false,
            lcdc: 0x91, // Default value post-boot ROM
            stat: 0x85, // Default value post-boot ROM (Mode 1 + LYC=LY)
        }
    }

    /// Resets the PPU state when the LCD is turned off.
    pub(super) fn reset_for_lcd_off(&mut self) {
        self.dots = 0;
        self.current_scanline = 0;
        // Set mode to HBLANK? Or VBLANK? Pandocs implies LY=0, Mode=0 when LCD off.
        self.ppu_mode = HBLANK_MODE;
        self.lyc_eq_ly = false;
        self.stat_interrupt_line = false;
        // Don't reset lcdc/stat caches here, they get updated from bus
    }

    /// Gets the current PPU mode.
    pub fn mode(&self) -> u8 {
        self.ppu_mode
    }

    /// Gets the current scanline (LY).
    pub fn scanline(&self) -> u8 {
        self.current_scanline
    }
}
