use sdl2::pixels::Color;
use std::time::Duration;
// Import ppu constants if they are made public in the library
// use gameboy_emulator::ppu;

// --- Helper Const Functions ---

/// Compile-time constant function to find the maximum of two u32 values.
/// Needed because std::cmp::max is not always usable in const context.
const fn const_max_u32(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

// --- Timing ---
pub const TARGET_FPS: u32 = 60;
// Note: The calculation `1_000_000_000 / TARGET_FPS` needs to be done carefully in const context.
// Ensure TARGET_FPS is not zero. Explicit u64 cast is good.
pub const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000u64 / TARGET_FPS as u64));
pub const CPU_FREQ_HZ: f64 = 4_194_304.0; // Standard Game Boy CPU frequency
// CYCLES_PER_FRAME calculation involves floats, cannot be const directly.
// Calculate this at runtime if needed, or define as a const based on known values.
// For 60 FPS and 4.194304 MHz, it's approx 69905 cycles.
pub const CYCLES_PER_FRAME: u32 = 69905; // Pre-calculated approximate value

// --- Screen & Scaling ---
pub const GB_WIDTH: u32 = 160;          // Native Game Boy screen width in pixels
pub const GB_HEIGHT: u32 = 144;         // Native Game Boy screen height in pixels
pub const GB_SCALE_FACTOR: u32 = 3;     // How much to scale the GB screen display
pub const GB_SCREEN_WIDTH: u32 = GB_WIDTH * GB_SCALE_FACTOR;
pub const GB_SCREEN_HEIGHT: u32 = GB_HEIGHT * GB_SCALE_FACTOR;

// --- VRAM Debug View ---
// NOTE: These depend on VRAM_DEBUG_WIDTH and VRAM_DEBUG_HEIGHT from the core PPU logic.
//       They should ideally be imported if made public in the ppu module,
//       otherwise defined here based on known PPU tile layout (384 tiles total).
const TILES_PER_ROW_DEBUG: usize = 16;
const NUM_TILES_TO_SHOW: usize = 384; // 0x8000-0x97FF (2 banks * 192 tiles)
const TILE_PIXEL_DIM: usize = 8; // Tiles are 8x8 pixels
const VRAM_DEBUG_NATIVE_WIDTH: u32 = (TILES_PER_ROW_DEBUG * TILE_PIXEL_DIM) as u32; // Width in native pixels (16*8 = 128)
const VRAM_DEBUG_NATIVE_HEIGHT: u32 = ((NUM_TILES_TO_SHOW / TILES_PER_ROW_DEBUG) * TILE_PIXEL_DIM) as u32; // Height in native pixels ( (384/16)*8 = 24*8 = 192)
pub const VRAM_DEBUG_SCALE_FACTOR: u32 = 2; // How much to scale the VRAM debug view
pub const VRAM_VIEW_WIDTH: u32 = VRAM_DEBUG_NATIVE_WIDTH * VRAM_DEBUG_SCALE_FACTOR; // 128 * 2 = 256
pub const VRAM_VIEW_HEIGHT: u32 = VRAM_DEBUG_NATIVE_HEIGHT * VRAM_DEBUG_SCALE_FACTOR; // 192 * 2 = 384


// --- General Debugging UI ---
pub const PADDING: u32 = 10; // Padding between UI elements
pub const DEBUG_BACKGROUND_COLOR: Color = Color::RGB(30, 30, 30); // Background for debug panes


// --- Input Debug ---
pub const DEBUG_INPUT_BOX_SIZE: u32 = 15;   // Size of the square indicator
pub const DEBUG_INPUT_PADDING: u32 = 4;     // Padding between input indicators
pub const DEBUG_INPUT_PRESSED_COLOR: Color = Color::RGB(50, 205, 50); // Lime Green
pub const DEBUG_INPUT_RELEASED_COLOR: Color = Color::RGB(70, 70, 70); // Dark Gray
// Calculated Input Debug Area Dimensions
pub const DPAD_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 3 + DEBUG_INPUT_PADDING * 2;
pub const DPAD_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 3 + DEBUG_INPUT_PADDING * 2;
pub const ACTION_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1; // A, B
pub const ACTION_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1; // Start, Select (stacked below A, B perhaps?) - Adjust if layout differs
// Let's assume Start/Select are *next* to A/B for width calculation
pub const BUTTONS_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1; // A, B side-by-side
pub const BUTTONS_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1; // Start, Select side-by-side

// Combine DPAD and Action Buttons horizontally with padding
pub const INPUT_DEBUG_AREA_WIDTH: u32 = DPAD_AREA_WIDTH + PADDING + BUTTONS_AREA_WIDTH;
// Height is the max of the Dpad area height and the Button area height
// Use the const_max_u32 helper function here
pub const INPUT_DEBUG_AREA_HEIGHT: u32 = const_max_u32(DPAD_AREA_HEIGHT, BUTTONS_AREA_HEIGHT);


// --- Disassembly Debug ---
// <<< --- IMPORTANT: CHANGE THIS PATH TO MATCH YOUR SYSTEM! --- >>>
// Consider using a relative path or embedding the font
pub const FONT_PATH: &str = "/home/ankan/GameBoy/core/src/app/Roboto-Regular.ttf"; // Example using relative path
// pub const FONT_PATH: &str = "/home/ankan/GameBoy/core/src/bin/Roboto-Regular.ttf"; // Original absolute path
pub const DEBUG_FONT_SIZE: u16 = 14;
pub const DISASM_LINES_BEFORE: usize = 5; // Lines to show before PC
pub const DISASM_LINES_AFTER: usize = 10; // Lines to show after PC
pub const DISASM_TOTAL_LINES: usize = DISASM_LINES_BEFORE + 1 + DISASM_LINES_AFTER;
pub const DISASM_LINE_HEIGHT: u32 = (DEBUG_FONT_SIZE + 4) as u32; // Height per disassembly line (add padding)
pub const DISASM_AREA_WIDTH: u32 = 350;   // Fixed width for the disassembly pane (adjust as needed)
pub const DISASM_AREA_HEIGHT: u32 = DISASM_LINE_HEIGHT * (DISASM_TOTAL_LINES as u32); // Calculated height
pub const DEBUG_PC_COLOR: Color = Color::RGB(255, 255, 0);     // Yellow for current PC line
pub const DEBUG_TEXT_COLOR: Color = Color::RGB(220, 220, 220); // Light Gray for text


// --- Palettes ---
pub const PALETTE: [Color; 4] = [
    Color::RGB(0x9B, 0xBC, 0x0F), // Lightest Green (Color 0 / White)
    Color::RGB(0x8B, 0xAC, 0x0F), // Light Green   (Color 1 / Light Gray)
    Color::RGB(0x30, 0x62, 0x30), // Dark Green    (Color 2 / Dark Gray)
    Color::RGB(0x0F, 0x38, 0x0F), // Darkest Green (Color 3 / Black)
];
// Palette used for the VRAM debug view (simple grayscale)
pub const DEBUG_PALETTE: [Color; 4] = [
    Color::RGB(0xFF, 0xFF, 0xFF), // White
    Color::RGB(0xAA, 0xAA, 0xAA), // Light Gray
    Color::RGB(0x55, 0x55, 0x55), // Dark Gray
    Color::RGB(0x00, 0x00, 0x00), // Black
];


// --- Window Layout Calculations ---
// This function calculates dimensions at RUNTIME.
// If you need compile-time window dimensions, you'd need to replicate this logic
// using only const operations and the `const_max_u32` helper.
// Keeping it as a function is fine if you call it once during initialization.
pub fn calculate_window_dims() -> (u32, u32) {
    // Define the widths of the three main columns
    let col1_width = GB_SCREEN_WIDTH;
    let col2_width = DISASM_AREA_WIDTH;
    // Column 3 contains VRAM view stacked above Input view, so its width is the max of the two.
    let col3_width = std::cmp::max(VRAM_VIEW_WIDTH, INPUT_DEBUG_AREA_WIDTH);

    // Calculate total window width based on the three columns and padding
    let total_window_width: u32 = col1_width + PADDING + col2_width + PADDING + col3_width;

    // Calculate the heights needed for each column/area
    let col1_height = GB_SCREEN_HEIGHT;
    let col2_height = DISASM_AREA_HEIGHT;
    // Column 3 height is VRAM + Padding + Input Debug
    let col3_height = VRAM_VIEW_HEIGHT + PADDING + INPUT_DEBUG_AREA_HEIGHT;

    // Total window height is the maximum height required by any of the effective vertical columns
    // (GB Screen, Disassembly, VRAM+Input Stack)
    let total_window_height: u32 = std::cmp::max(col1_height, std::cmp::max(col2_height, col3_height));

    (total_window_width, total_window_height)
}