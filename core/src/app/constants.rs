use sdl2::pixels::Color;
use std::time::Duration;
// Import constants from the core library (assuming 'boba' is your core crate name)
// Make sure these are declared as `pub const` in boba::ppu
pub use boba::ppu::{GB_HEIGHT, GB_WIDTH, VRAM_DEBUG_HEIGHT as PPU_VRAM_DEBUG_NATIVE_HEIGHT, VRAM_DEBUG_WIDTH as PPU_VRAM_DEBUG_NATIVE_WIDTH};

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
pub const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000u64 / TARGET_FPS as u64));
pub const CPU_FREQ_HZ: f64 = 4_194_304.0; // Standard Game Boy CPU frequency
// For 60 FPS and 4.194304 MHz, it's approx 69905 cycles.
pub const CYCLES_PER_FRAME: u32 = 69905; // Pre-calculated approximate value

// --- Screen & Scaling ---
// GB_WIDTH and GB_HEIGHT are now imported from boba::ppu
pub const GB_SCALE_FACTOR: u32 = 3;     // How much to scale the GB screen display
pub const GB_SCREEN_WIDTH: u32 = GB_WIDTH as u32 * GB_SCALE_FACTOR; // Uses imported GB_WIDTH
pub const GB_SCREEN_HEIGHT: u32 = GB_HEIGHT as u32 * GB_SCALE_FACTOR; // Uses imported GB_HEIGHT

// --- VRAM Debug View ---
// The native dimensions are now imported from boba::ppu and aliased for clarity
// PPU_VRAM_DEBUG_NATIVE_WIDTH (e.g., 128)
// PPU_VRAM_DEBUG_NATIVE_HEIGHT (e.g., 192)
pub const VRAM_DEBUG_SCALE_FACTOR: u32 = 2; // How much to scale the VRAM debug view for display
// Scaled dimensions for the view pane, based on imported native dimensions
pub const VRAM_VIEW_WIDTH: u32 = PPU_VRAM_DEBUG_NATIVE_WIDTH as u32 * VRAM_DEBUG_SCALE_FACTOR;
pub const VRAM_VIEW_HEIGHT: u32 = PPU_VRAM_DEBUG_NATIVE_HEIGHT as u32 * VRAM_DEBUG_SCALE_FACTOR;


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
// Assuming A/B side-by-side and Start/Select side-by-side
pub const BUTTONS_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1;
pub const BUTTONS_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1;

// Combine DPAD and Action Buttons horizontally with padding
pub const INPUT_DEBUG_AREA_WIDTH: u32 = DPAD_AREA_WIDTH + PADDING + BUTTONS_AREA_WIDTH;
// Height is the max of the Dpad area height and the Button area height
// Use the const_max_u32 helper function here
pub const INPUT_DEBUG_AREA_HEIGHT: u32 = const_max_u32(DPAD_AREA_HEIGHT, BUTTONS_AREA_HEIGHT);


// --- Disassembly Debug ---
// <<< --- RECOMMEND using a relative path or embedding --- >>>
// Example relative path (assumes an 'assets/fonts' directory next to 'src')
pub const FONT_PATH: &str = "/home/ankan/GameBoy/core/src/app/Roboto-Regular.ttf";
// Your original absolute path:
// pub const FONT_PATH: &str = "/home/ankan/GameBoy/core/src/app/Roboto-Regular.ttf";
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
// This function calculates dimensions at RUNTIME using the const values defined above.
pub fn calculate_window_dims() -> (u32, u32) {
    // Define the widths of the three main columns
    let col1_width = GB_SCREEN_WIDTH; // Scaled GB screen width
    let col2_width = DISASM_AREA_WIDTH; // Disassembly pane width
    // Column 3 contains VRAM view stacked above Input view, so its width is the max of the two panes.
    let col3_width = std::cmp::max(VRAM_VIEW_WIDTH, INPUT_DEBUG_AREA_WIDTH); // Use scaled VRAM width

    // Calculate total window width based on the three columns and padding
    let total_window_width: u32 = col1_width + PADDING + col2_width + PADDING + col3_width;

    // Calculate the heights needed for each column/area
    let col1_height = GB_SCREEN_HEIGHT; // Scaled GB screen height
    let col2_height = DISASM_AREA_HEIGHT; // Disassembly pane height
    // Column 3 height is Scaled VRAM + Padding + Input Debug height
    let col3_height = VRAM_VIEW_HEIGHT + PADDING + INPUT_DEBUG_AREA_HEIGHT;

    // Total window height is the maximum height required by any of the effective vertical columns
    // (GB Screen, Disassembly, VRAM+Input Stack)
    let total_window_height: u32 = std::cmp::max(col1_height, std::cmp::max(col2_height, col3_height));

    (total_window_width, total_window_height)
}