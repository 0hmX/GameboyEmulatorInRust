use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton; // Import MouseButton for mouse events
use sdl2::render::Canvas;
use sdl2::video::Window;

use std::time::{Duration, Instant};
use std::thread;
use std::env;
use std::path::Path;
use std::cmp; // Import cmp for max calculation

// --- Emulator Core Imports ---
// Adjust path based on your project structure (e.g., `crate::` if in the same crate)
use gameboy_emulator::memory_bus::{MemoryBus, JoypadState}; // Ensure JoypadState is accessible
use gameboy_emulator::cpu::Cpu;
// Import Ppu and the *public* debug constants
use gameboy_emulator::ppu::{Ppu, VRAM_DEBUG_WIDTH, VRAM_DEBUG_HEIGHT};

// --- Timing Constants ---
const TARGET_FPS: u32 = 60;
const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000 / TARGET_FPS) as u64);

// --- CPU/Emulator Constants ---
const CPU_FREQ_HZ: f64 = 4_194_304.0; // 4.194304 MHz (dot clock frequency)
// Cycles per frame based on CPU frequency and target FPS
const CYCLES_PER_FRAME: u32 = (CPU_FREQ_HZ / TARGET_FPS as f64) as u32;

// --- Screen/Drawing Constants ---
const GB_WIDTH: u32 = 160;
const GB_HEIGHT: u32 = 144;
const GB_SCALE_FACTOR: u32 = 4; // Scale factor for the main GB screen
const VRAM_DEBUG_SCALE_FACTOR: u32 = 2; // Scale factor for the VRAM debug view
const PADDING: u32 = 10; // Pixels between views

// --- Input Debug Visualizer Constants ---
const DEBUG_INPUT_BOX_SIZE: u32 = 20;
const DEBUG_INPUT_PADDING: u32 = 5;
const DEBUG_INPUT_PRESSED_COLOR: Color = Color::RGB(50, 205, 50); // Lime Green
const DEBUG_INPUT_RELEASED_COLOR: Color = Color::RGB(70, 70, 70); // Dark Gray

// --- Calculate Constant Dimensions ---
// Dimensions that don't rely on `cmp::max` can be const
const GB_SCREEN_WIDTH: u32 = GB_WIDTH * GB_SCALE_FACTOR;
const GB_SCREEN_HEIGHT: u32 = GB_HEIGHT * GB_SCALE_FACTOR;
// Use the imported constants correctly (they are usize, cast to u32)
const VRAM_VIEW_WIDTH: u32 = VRAM_DEBUG_WIDTH as u32 * VRAM_DEBUG_SCALE_FACTOR;
const VRAM_VIEW_HEIGHT: u32 = VRAM_DEBUG_HEIGHT as u32 * VRAM_DEBUG_SCALE_FACTOR;

// Input Debug Area size components that *can* be const
const DPAD_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 3 + DEBUG_INPUT_PADDING * 2;
const DPAD_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 3 + DEBUG_INPUT_PADDING * 2;
const ACTION_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1;
const ACTION_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1;
const INPUT_DEBUG_AREA_WIDTH: u32 = DPAD_AREA_WIDTH + PADDING + ACTION_AREA_WIDTH;

// --- Palettes ---
// Define the Game Boy grayscale palette (common greenish one)
const PALETTE: [Color; 4] = [
    Color::RGB(0x9B, 0xBC, 0x0F), // 0: Lightest Green/White
    Color::RGB(0x8B, 0xAC, 0x0F), // 1: Light Gray/Green
    Color::RGB(0x30, 0x62, 0x30), // 2: Dark Gray/Green
    Color::RGB(0x0F, 0x38, 0x0F), // 3: Black/Darkest Green
];

// Simple Grayscale Palette for VRAM Debug view
const DEBUG_PALETTE: [Color; 4] = [
    Color::RGB(0xFF, 0xFF, 0xFF), // 0: White
    Color::RGB(0xAA, 0xAA, 0xAA), // 1: Light Gray
    Color::RGB(0x55, 0x55, 0x55), // 2: Dark Gray
    Color::RGB(0x00, 0x00, 0x00), // 3: Black
];


// --- Drawing Helper Functions ---

/// Draws the Game Boy frame buffer to a specific area of the SDL2 canvas.
fn draw_gb_screen(canvas: &mut Canvas<Window>, frame_buffer: &[u8], target_x: i32, target_y: i32) {
    if frame_buffer.len() != (GB_WIDTH * GB_HEIGHT) as usize {
        eprintln!("Error: Frame buffer size mismatch! Expected {}, got {}", (GB_WIDTH * GB_HEIGHT), frame_buffer.len());
        return;
    }
    for y in 0..GB_HEIGHT {
        for x in 0..GB_WIDTH {
            let index = (y * GB_WIDTH + x) as usize;
            // Use modulo 4 to handle potential invalid shade indices gracefully
            let shade_index = frame_buffer.get(index).copied().unwrap_or(0) % 4;
            let color = PALETTE[shade_index as usize];

            canvas.set_draw_color(color);
            let rect = Rect::new(
                target_x + (x * GB_SCALE_FACTOR) as i32,
                target_y + (y * GB_SCALE_FACTOR) as i32,
                GB_SCALE_FACTOR, // Width of scaled pixel
                GB_SCALE_FACTOR, // Height of scaled pixel
            );
            // Using unwrap - if fill_rect fails, SDL setup is likely broken.
            canvas.fill_rect(rect).unwrap();
        }
    }
}

/// Draws the VRAM debug buffer to a specific area of the SDL2 canvas.
fn draw_vram_debug(canvas: &mut Canvas<Window>, vram_buffer: &[u8], target_x: i32, target_y: i32) {
    let expected_len = VRAM_DEBUG_WIDTH * VRAM_DEBUG_HEIGHT;
     if vram_buffer.len() != expected_len {
         eprintln!("Error: VRAM debug buffer size mismatch! Expected {}, got {}", expected_len, vram_buffer.len());
         return;
     }
    for y in 0..VRAM_DEBUG_HEIGHT { // Iterate using the usize dimension from PPU const
        for x in 0..VRAM_DEBUG_WIDTH { // Iterate using the usize dimension from PPU const
            let index = y * VRAM_DEBUG_WIDTH + x;
            // Use modulo 4 for safety
            let shade_index = vram_buffer.get(index).copied().unwrap_or(0) % 4;
            let color = DEBUG_PALETTE[shade_index as usize];

            canvas.set_draw_color(color);
            let rect = Rect::new(
                target_x + (x as u32 * VRAM_DEBUG_SCALE_FACTOR) as i32,
                target_y + (y as u32 * VRAM_DEBUG_SCALE_FACTOR) as i32,
                VRAM_DEBUG_SCALE_FACTOR, // Width of scaled pixel
                VRAM_DEBUG_SCALE_FACTOR, // Height of scaled pixel
            );
             canvas.fill_rect(rect).unwrap();
        }
    }
}

/// Draws the input state visualizer using rectangles.
/// Assumes access to `JoypadState` via `bus.joypad`.
fn draw_input_debug(canvas: &mut Canvas<Window>, joypad_state: &JoypadState, target_x: i32, target_y: i32) {

    // Helper closure to draw one indicator box
    let mut draw_indicator = |is_pressed: bool, x_offset: i32, y_offset: i32| {
        let color = if is_pressed { DEBUG_INPUT_PRESSED_COLOR } else { DEBUG_INPUT_RELEASED_COLOR };
        canvas.set_draw_color(color);
        let rect = Rect::new(
            target_x + x_offset,
            target_y + y_offset,
            DEBUG_INPUT_BOX_SIZE,
            DEBUG_INPUT_BOX_SIZE,
        );
        canvas.fill_rect(rect).unwrap();
    };

    // --- Draw D-Pad (visual 3x3 grid layout) ---
    // Calculate offsets relative to target_x, target_y
    let pad_step = (DEBUG_INPUT_BOX_SIZE + DEBUG_INPUT_PADDING) as i32;
    let dpad_center_x = pad_step; // Center X within the DPad area relative to target_x
    let dpad_center_y = pad_step; // Center Y within the DPad area relative to target_y

    // Draw indicators based on joypad state
    draw_indicator(joypad_state.up, dpad_center_x, dpad_center_y - pad_step); // Up
    draw_indicator(joypad_state.down, dpad_center_x, dpad_center_y + pad_step); // Down
    draw_indicator(joypad_state.left, dpad_center_x - pad_step, dpad_center_y); // Left
    draw_indicator(joypad_state.right, dpad_center_x + pad_step, dpad_center_y); // Right

    // --- Draw Action Buttons (Positioned to the right of the D-Pad area) ---
    let action_start_x = DPAD_AREA_WIDTH as i32 + PADDING as i32; // Start X relative to target_x
    let action_y1 = 0; // Top row Y relative to target_y
    let action_y2 = pad_step; // Bottom row Y relative to target_y

    // Draw indicators based on joypad state
    draw_indicator(joypad_state.b, action_start_x, action_y1); // B (Top Left)
    draw_indicator(joypad_state.a, action_start_x + pad_step, action_y1); // A (Top Right)
    draw_indicator(joypad_state.select, action_start_x, action_y2); // Select (Bottom Left)
    draw_indicator(joypad_state.start, action_start_x + pad_step, action_y2); // Start (Bottom Right)
}


// --- Main Function ---
pub fn main() -> Result<(), String> {
    // --- Argument Parsing ---
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_path>", args[0]);
        std::process::exit(1);
    }
    let rom_path = Path::new(&args[1]);

    // --- SDL Initialization ---
    println!("Initializing SDL2...");
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window_title = format!("Rust GB Emu - {}", rom_path.file_name().unwrap_or_default().to_string_lossy());

    // --- Calculate Window Dimensions at Runtime ---
    // Calculate height needed for the right pane (VRAM + Input Debug)
    let input_debug_area_height: u32 = cmp::max(DPAD_AREA_HEIGHT, ACTION_AREA_HEIGHT);
    let right_pane_height = VRAM_VIEW_HEIGHT + PADDING + input_debug_area_height;
    // Total height is max of GB screen height and right pane height
    let total_window_height: u32 = cmp::max(GB_SCREEN_HEIGHT, right_pane_height);
    // Total width includes GB screen, padding, and the wider of VRAM/Input debug views
    let total_window_width: u32 = GB_SCREEN_WIDTH + PADDING + cmp::max(VRAM_VIEW_WIDTH, INPUT_DEBUG_AREA_WIDTH);

    println!("Creating window ({}x{})...", total_window_width, total_window_height);
    let window = video_subsystem
        .window(&window_title, total_window_width, total_window_height) // Use calculated dimensions
        .position_centered()
        .build() // Creates the window builder
        .map_err(|e| e.to_string())?; // Handles potential errors during build

    println!("Creating accelerated canvas...");
    let mut canvas = window.into_canvas()
        .accelerated() // Request hardware acceleration if available
        // .present_vsync() // Disable VSync for manual frame rate control
        .build() // Creates the canvas
        .map_err(|e| e.to_string())?; // Handles potential errors

    println!("Initializing event pump...");
    let mut event_pump = sdl_context.event_pump()?;

    // --- Emulator Initialization ---
    println!("Initializing memory bus...");
    let mut memory_bus = MemoryBus::new(); // Creates the memory bus

    println!("Loading ROM: {}", rom_path.display());
    match std::fs::read(rom_path) {
        Ok(rom_data) => {
            // Load ROM data into the memory bus
            memory_bus.load_rom(&rom_data);
            println!("ROM loaded successfully ({} bytes)", rom_data.len());
        }
        Err(e) => {
            // Handle ROM loading errors gracefully
             eprintln!("FATAL: Failed to load ROM '{}': {}", rom_path.display(), e);
             return Err(format!("Failed to load ROM: {}", e));
         }
    }

    // --- Boot ROM Configuration ---
    // Set to true to skip the GB boot ROM and jump directly into the game.
    // Set to false if your MemoryBus loads a boot ROM and you want to run it.
    let skip_boot_rom = true;

    println!("Initializing CPU (skip_boot_rom={})...", skip_boot_rom);
    let mut cpu = Cpu::new(skip_boot_rom); // Create the CPU instance

    if skip_boot_rom {
        println!("Skipping boot ROM - initializing I/O registers post-boot...");
        // If skipping, manually set up necessary CPU/IO state
        Cpu::initialize_post_boot_io(&mut memory_bus);
    } else {
        println!("Running boot ROM (ensure boot ROM is loaded in MemoryBus)...");
        // If running boot ROM, CPU PC starts at 0x0000 and executes it.
    }

    println!("Initializing PPU...");
    let mut ppu = Ppu::new(); // Create the PPU instance

    // --- Main Emulator Loop ---
    println!("Starting main loop...");
    'main_loop: loop {
        let frame_start_time = Instant::now(); // Mark start time for frame rate limiting

        // --- 1. Handle Input ---
        for event in event_pump.poll_iter() { // Process all pending events
            match event {
                // --- Quit Events ---
                Event::Quit { .. } => {
                    println!("Quit event received. Exiting loop.");
                    break 'main_loop;
                }
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Escape key pressed. Exiting loop.");
                    break 'main_loop;
                }

                // --- Keyboard Input for Game Boy ---
                Event::KeyDown { keycode: Some(key), repeat: false, .. } => {
                    // Pass key press to the memory bus to update Joypad state
                    memory_bus.key_down(key);
                }
                Event::KeyUp { keycode: Some(key), repeat: false, .. } => {
                    // Pass key release to the memory bus
                    memory_bus.key_up(key);
                }

                // --- Optional Mouse Event Handling (Placeholders) ---
                Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                    // Example: Log mouse clicks for debugging UI interactions
                    // println!("Mouse Down: {:?} at ({}, {})", mouse_btn, x, y);
                }
                Event::MouseButtonUp { .. } => {}
                Event::MouseMotion { .. } => {}
                Event::MouseWheel { .. } => {}

                // Ignore other events (like window focus, text input unless needed)
                _ => {}
            }
        }

        // --- 2. Emulate One Frame ---
        let mut cycles_this_frame: u32 = 0;
        // Run CPU and PPU (and other components) for the target number of cycles
        while cycles_this_frame < CYCLES_PER_FRAME {
            // Execute one CPU step (instruction), returns cycles consumed
            let executed_cycles = cpu.step(&mut memory_bus) as u32;

            // Step the PPU by the number of cycles the CPU took
            ppu.step(executed_cycles, &mut memory_bus);

            // TODO: Step Timer component using executed_cycles
            // e.g., memory_bus.timer_step(executed_cycles); or timer.step(...)

            // TODO: Step APU (Sound) component using executed_cycles

            cycles_this_frame += executed_cycles;

            // Interrupt handling logic should be within cpu.step() checking IF/IE
        }

        // --- 3. Update Debug Views ---
        // Render VRAM content into the PPU's debug buffer
        ppu.render_vram_debug(&memory_bus);

        // --- 4. Drawing ---
        // Clear the entire screen with a background color
        canvas.set_draw_color(Color::RGB(20, 20, 20)); // Dark gray
        canvas.clear();

        // Draw the main Game Boy screen area (Top-Left)
        draw_gb_screen(&mut canvas, ppu.get_frame_buffer(), 0, 0);

        // Calculate position for the VRAM debug view (Top-Right pane)
        let vram_view_x = (GB_SCREEN_WIDTH + PADDING) as i32;
        let vram_view_y = 0;
        // Draw the VRAM debug content
        draw_vram_debug(&mut canvas, ppu.get_vram_debug_buffer(), vram_view_x, vram_view_y);

        // Calculate position for the Input debug view (Right pane, below VRAM)
        let input_view_x = vram_view_x;
        let input_view_y = (VRAM_VIEW_HEIGHT + PADDING) as i32;
        // Draw the input state visualizer using the public joypad state
        draw_input_debug(&mut canvas, &memory_bus.joypad, input_view_x, input_view_y);

        // Present the drawn frame to the window
        canvas.present();

        // --- 5. Frame Timing / Rate Limiting ---
        let elapsed_time = frame_start_time.elapsed(); // Calculate time spent this frame
        // If frame finished faster than target, wait for the remaining time
        if elapsed_time < TARGET_FRAME_DURATION {
            let sleep_duration = TARGET_FRAME_DURATION.saturating_sub(elapsed_time);
            // Use thread::sleep for longer waits to avoid busy-waiting excessively
            if sleep_duration > Duration::from_millis(1) {
                thread::sleep(sleep_duration.saturating_sub(Duration::from_millis(1)));
            }
            // Use yield or spin_loop for the final microseconds for better accuracy
            while Instant::now() < frame_start_time + TARGET_FRAME_DURATION {
                thread::yield_now(); // Yield CPU time slice
                // Alternatively: std::hint::spin_loop(); if preferred in very tight loops
            }
        }
        // Optional: Log if the frame took longer than the target duration
        // else {
        //     println!("Frame took too long: {:?}", elapsed_time);
        // }

    } // --- End of main_loop ---

    println!("Emulator stopped.");
    Ok(()) // Indicate successful execution
} // --- End of main ---