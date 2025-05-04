// src/ppu/constants.rs

#![allow(dead_code)] // Allow unused constants for definition completeness

// --- Screen Dimensions ---
pub const GB_WIDTH: usize = 160;
pub const GB_HEIGHT: usize = 144;
pub const FRAME_BUFFER_SIZE: usize = GB_WIDTH * GB_HEIGHT;

// --- VRAM Debug View Constants ---
pub const TILES_PER_ROW_DEBUG: usize = 16;
pub const NUM_TILES_TO_SHOW: usize = 384; // 256 tiles in $8000-$8FFF, 128 in $9000-$97FF
const VRAM_DEBUG_TILE_HEIGHT: usize = NUM_TILES_TO_SHOW / TILES_PER_ROW_DEBUG;
pub const VRAM_DEBUG_WIDTH: usize = TILES_PER_ROW_DEBUG * 8;
pub const VRAM_DEBUG_HEIGHT: usize = VRAM_DEBUG_TILE_HEIGHT * 8;
pub const VRAM_DEBUG_BUFFER_SIZE: usize = VRAM_DEBUG_WIDTH * VRAM_DEBUG_HEIGHT;

// --- PPU Timing Constants (in T-cycles) ---
pub const DOTS_PER_SCANLINE: u32 = 456;
pub const SCANLINES_PER_FRAME: u8 = 154; // 144 visible + 10 VBlank

// Mode Durations (approximate, Mode 3 varies slightly)
pub const MODE2_OAM_SCAN_DOTS: u32 = 80;
pub const MODE3_VRAM_READ_DOTS: u32 = 172; // Minimum duration
// Mode 0 duration varies based on Mode 3, ensures total is DOTS_PER_SCANLINE
// MODE0_HBLANK_DOTS = DOTS_PER_SCANLINE - MODE2_OAM_SCAN_DOTS - MODE3_VRAM_READ_DOTS

// --- PPU Modes (Values for STAT register bits 0-1) ---
pub const HBLANK_MODE: u8 = 0;
pub const VBLANK_MODE: u8 = 1;
pub const OAM_SCAN_MODE: u8 = 2;
pub const VRAM_READ_MODE: u8 = 3;

// --- Relevant Memory Addresses ---
// These are often also defined in memory_map.rs, use those preferably to avoid duplication.
// We might keep them here temporarily or ensure consistency.
// pub use crate::memory_map::{VRAM_START, VRAM_END, OAM_START, OAM_END};
// pub use crate::memory_map::{LCDC_ADDR, STAT_ADDR, SCY_ADDR, SCX_ADDR, LY_ADDR, LYC_ADDR};
// pub use crate::memory_map::{BGP_ADDR, OBP0_ADDR, OBP1_ADDR, WY_ADDR, WX_ADDR};
// pub use crate::memory_map::{IF_ADDR};
// pub use crate::memory_map::{VBLANK_INTERRUPT_BIT, LCD_STAT_INTERRUPT_BIT};

// --- LCDC Flags (Bit positions in LCDC register 0xFF40) ---
pub const LCDC_BG_WIN_ENABLE_PRIORITY: u8 = 0; // DMG: BG display enable; CGB: BG/Win priority
pub const LCDC_OBJ_ENABLE: u8 = 1; // Sprite display enable
pub const LCDC_OBJ_SIZE: u8 = 2; // Sprite size (0=8x8, 1=8x16)
pub const LCDC_BG_MAP_AREA: u8 = 3; // BG tile map area (0=9800-9BFF, 1=9C00-9FFF)
pub const LCDC_TILE_DATA_AREA: u8 = 4; // BG & Window tile data area (0=8800-97FF, 1=8000-8FFF)
pub const LCDC_WINDOW_ENABLE: u8 = 5; // Window display enable
pub const LCDC_WINDOW_MAP_AREA: u8 = 6; // Window tile map area (0=9800-9BFF, 1=9C00-9FFF)
pub const LCDC_LCD_ENABLE: u8 = 7; // LCD display enable (master on/off)

// --- STAT Flags (Bit positions in STAT register 0xFF41) ---
// Bits 0-1: Mode Flag (Read Only) - Matches PPU Modes above
pub const STAT_MODE_FLAG_0: u8 = 0;
pub const STAT_MODE_FLAG_1: u8 = 1;
pub const STAT_LYC_EQ_LY_FLAG: u8 = 2; // Coincidence Flag (Read Only)
// Bits 3-6: Interrupt Enables (Read/Write)
pub const STAT_MODE_0_HBLANK_IE: u8 = 3; // Mode 0 HBlank Interrupt Enable
pub const STAT_MODE_1_VBLANK_IE: u8 = 4; // Mode 1 VBlank Interrupt Enable
pub const STAT_MODE_2_OAM_IE: u8 = 5; // Mode 2 OAM Scan Interrupt Enable
pub const STAT_LYC_EQ_LY_IE: u8 = 6; // LYC=LY Coincidence Interrupt Enable
// Bit 7: Unused (Always reads 1)

// --- OAM Attribute Flags (Bit positions in OAM byte 3) ---
pub const OAM_PALETTE_NUM_DMG: u8 = 4; // DMG: Palette Number (0=OBP0, 1=OBP1)
pub const OAM_X_FLIP: u8 = 5; // Horizontal Flip (0=Normal, 1=Flipped)
pub const OAM_Y_FLIP: u8 = 6; // Vertical Flip (0=Normal, 1=Flipped)
pub const OAM_BG_WIN_PRIORITY: u8 = 7; // BG/Window Priority (0=Sprite above BG/Win, 1=Sprite behind BG colors 1-3)
