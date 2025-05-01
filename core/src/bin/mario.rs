use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;

use std::time::{Duration, Instant};
use std::thread;
use std::env;
use std::path::Path;
use std::cmp; // Import cmp for max

// Assuming these are in the lib.rs or similar of your crate
use gameboy_emulator::memory_bus::MemoryBus;
use gameboy_emulator::cpu::Cpu;
// Import Ppu and the *public* debug constants
use gameboy_emulator::ppu::{Ppu, VRAM_DEBUG_WIDTH, VRAM_DEBUG_HEIGHT};

// --- Constants ---
const TARGET_FPS: u32 = 60;
const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000 / TARGET_FPS) as u64);

const CPU_FREQ_HZ: f64 = 4_194_304.0; // 4.194304 MHz
const CYCLES_PER_FRAME: u32 = (CPU_FREQ_HZ / TARGET_FPS as f64) as u32;

// Screen/Drawing Constants
const GB_WIDTH: u32 = 160;
const GB_HEIGHT: u32 = 144;
const GB_SCALE_FACTOR: u32 = 4; // Scale factor for the main GB screen
const VRAM_DEBUG_SCALE_FACTOR: u32 = 4; // Scale factor for the VRAM debug view
const PADDING: u32 = 10; // Pixels between GB screen and VRAM view

// Calculate dimensions that *can* be const
const GB_SCREEN_WIDTH: u32 = GB_WIDTH * GB_SCALE_FACTOR;
const GB_SCREEN_HEIGHT: u32 = GB_HEIGHT * GB_SCALE_FACTOR;
// Use the imported constants correctly (they are usize, cast to u32)
const VRAM_VIEW_WIDTH: u32 = VRAM_DEBUG_WIDTH as u32 * VRAM_DEBUG_SCALE_FACTOR;
const VRAM_VIEW_HEIGHT: u32 = VRAM_DEBUG_HEIGHT as u32 * VRAM_DEBUG_SCALE_FACTOR;

// Total window width is constant
const TOTAL_WINDOW_WIDTH: u32 = GB_SCREEN_WIDTH + PADDING + VRAM_VIEW_WIDTH;
// Total window height will be calculated at runtime

// Define the Game Boy grayscale palette
const PALETTE: [Color; 4] = [
    Color::RGB(0x9B, 0xBC, 0x0F), // 0: Lightest Green/White
    Color::RGB(0x8B, 0xAC, 0x0F), // 1: Light Gray/Green
    Color::RGB(0x30, 0x62, 0x30), // 2: Dark Gray/Green
    Color::RGB(0x0F, 0x38, 0x0F), // 3: Black/Darkest Green
];

// Simple Grayscale Palette for Debug (Could use main palette too)
const DEBUG_PALETTE: [Color; 4] = [
    Color::RGB(0xFF, 0xFF, 0xFF), // 0: White
    Color::RGB(0xAA, 0xAA, 0xAA), // 1: Light Gray
    Color::RGB(0x55, 0x55, 0x55), // 2: Dark Gray
    Color::RGB(0x00, 0x00, 0x00), // 3: Black
];


/// Draws the Game Boy frame buffer to a specific area of the SDL2 canvas.
/// `frame_buffer` should be a slice containing 160*144 bytes (shade 0-3).
/// `canvas`: The target canvas.
/// `target_x`: Top-left X coordinate on the canvas.
/// `target_y`: Top-left Y coordinate on the canvas.
fn draw_gb_screen(canvas: &mut Canvas<Window>, frame_buffer: &[u8], target_x: i32, target_y: i32) {
    if frame_buffer.len() != (GB_WIDTH * GB_HEIGHT) as usize {
        eprintln!("Error: Frame buffer size mismatch! Expected {}, got {}", (GB_WIDTH * GB_HEIGHT), frame_buffer.len());
        return; // Don't attempt to draw if buffer is wrong size
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
            // Using unwrap - if fill_rect fails, something is very wrong.
            canvas.fill_rect(rect).unwrap();
        }
    }
}

/// Draws the VRAM debug buffer to a specific area of the SDL2 canvas.
/// `vram_buffer` should be a slice containing VRAM_DEBUG_WIDTH * VRAM_DEBUG_HEIGHT bytes (shade 0-3).
/// `canvas`: The target canvas.
/// `target_x`: The top-left X coordinate on the canvas where the VRAM view should start.
/// `target_y`: The top-left Y coordinate on the canvas where the VRAM view should start.
fn draw_vram_debug(canvas: &mut Canvas<Window>, vram_buffer: &[u8], target_x: i32, target_y: i32) {
    let expected_len = VRAM_DEBUG_WIDTH * VRAM_DEBUG_HEIGHT;
     if vram_buffer.len() != expected_len {
         eprintln!("Error: VRAM debug buffer size mismatch! Expected {}, got {}", expected_len, vram_buffer.len());
         return; // Don't attempt to draw if buffer is wrong size
     }

    for y in 0..VRAM_DEBUG_HEIGHT { // Iterate using the usize dimension
        for x in 0..VRAM_DEBUG_WIDTH { // Iterate using the usize dimension
            let index = y * VRAM_DEBUG_WIDTH + x;
            // Use modulo 4 for safety
            let shade_index = vram_buffer.get(index).copied().unwrap_or(0) % 4;
            // Use DEBUG_PALETTE for the VRAM view
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


pub fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_path>", args[0]);
        std::process::exit(1);
    }
    let rom_path = Path::new(&args[1]);

    // --- SDL Initialization ---
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window_title = format!("Rust GB Emu - {}", rom_path.file_name().unwrap_or_default().to_string_lossy());

    // --- Calculate total window height at runtime ---
    let total_window_height: u32 = cmp::max(GB_SCREEN_HEIGHT, VRAM_VIEW_HEIGHT);

    let window = video_subsystem
        .window(&window_title, TOTAL_WINDOW_WIDTH, total_window_height) // Use calculated height
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas()
        .accelerated() // Request hardware acceleration
        //.present_vsync() // Disable vsync for manual frame timing
        .build()
        .map_err(|e| e.to_string())?;

    let mut event_pump = sdl_context.event_pump()?;

    // --- Emulator Initialization ---
    println!("Initializing memory bus...");
    let mut memory_bus = MemoryBus::new(); // MemoryBus owns RAM, Cartridge, I/O state

    println!("Loading ROM: {}", rom_path.display());
    match std::fs::read(rom_path) {
        Ok(rom_data) => {
            // Assume load_rom handles putting the data into the MemoryBus's cartridge area
            memory_bus.load_rom(&rom_data);
            println!("ROM loaded successfully ({} bytes)", rom_data.len());
        }
        Err(e) => {
             eprintln!("Failed to load ROM '{}': {}", rom_path.display(), e);
             return Err(format!("Failed to load ROM: {}", e)); // Exit if ROM fails
         }
    }

    // --- Boot ROM Skipping ---
    // Set to false if MemoryBus::new() loads a boot ROM and handles mapping
    // Set to true to jump directly into game execution state
    let skip_boot_rom = true;

    println!("Initializing CPU (skip_boot_rom={})...", skip_boot_rom);
    let mut cpu = Cpu::new(skip_boot_rom); // CPU gets MemoryBus passed during step

    if skip_boot_rom {
        println!("Skipping boot ROM - initializing I/O registers...");
        // Ensure this function correctly sets up PC, SP, and critical I/O regs
        // like LCDC, STAT, IF, IE etc. within the memory_bus
        Cpu::initialize_post_boot_io(&mut memory_bus);
    } else {
        println!("Running boot ROM (ensure it's loaded in MemoryBus)...");
        // Boot ROM execution starts automatically if skip_boot_rom is false
        // and CPU PC is initialized to 0x0000. MemoryBus must handle mapping.
    }

    println!("Initializing PPU...");
    let mut ppu = Ppu::new(); // Instantiate the PPU

    println!("Starting main loop... Target FPS: {}, Cycles/Frame: {}", TARGET_FPS, CYCLES_PER_FRAME);
    'main_loop: loop {
        let frame_start_time = Instant::now();

        // --- Handle Input ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Exiting main loop.");
                    break 'main_loop;
                }
                 Event::KeyDown { keycode: Some(key), repeat: false, .. } => {
                    // Assuming memory_bus has a public joypad field or a method
                    // memory_bus.joypad.handle_key_down(key);
                    // Need to potentially request a Joypad interrupt if enabled bits match
                    // memory_bus.request_interrupt(4); // Joypad is bit 4 in IF/IE
                 }
                 Event::KeyUp { keycode: Some(key), repeat: false, .. } => {
                     // Assuming memory_bus has a public joypad field or a method
                    //  memory_bus.joypad.handle_key_up(key);
                 }
                _ => {}
            }
        }

        // --- Emulate one frame ---
        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            // CPU executes instruction(s), returns cycles used. Interacts with MemoryBus.
            let executed_cycles = cpu.step(&mut memory_bus) as u32;

            // PPU steps forward based on CPU cycles. Reads/Writes MemoryBus (VRAM, OAM, IO Regs, IF).
            ppu.step(executed_cycles, &mut memory_bus);

            // TODO: Step Timer - Timer needs to step based on cycles and access MemoryBus for DIV/TIMA/TMA/TAC regs and IF
            // memory_bus.timer_step(executed_cycles); // Example: maybe timer logic is in MemoryBus

            // TODO: Step APU (Sound) - APU needs cycles and MemoryBus access

            cycles_this_frame += executed_cycles;

            // Interrupts are checked and handled *inside* cpu.step() by reading IF/IE from MemoryBus.
        }

        // --- Update Debug Views (e.g., VRAM) ---
        // This reads VRAM via MemoryBus and updates the PPU's internal debug buffer.
        ppu.render_vram_debug(&memory_bus);

        // --- Drawing ---
        // 1. Clear the entire canvas (e.g., black or a neutral color)
        canvas.set_draw_color(Color::RGB(20, 20, 20)); // Dark gray background
        canvas.clear();

        // 2. Draw the main Game Boy screen
        let frame_to_draw = ppu.get_frame_buffer();
        draw_gb_screen(&mut canvas, frame_to_draw, 0, 0); // Draw at top-left (0, 0)

        // 3. Draw the VRAM Debug view to the right
        let vram_to_draw = ppu.get_vram_debug_buffer();
        // Calculate the top-left corner for the VRAM view
        let vram_view_x = (GB_SCREEN_WIDTH + PADDING) as i32;
        let vram_view_y = 0; // Align to top
        draw_vram_debug(&mut canvas, vram_to_draw, vram_view_x, vram_view_y);

        // 4. Present the complete canvas to the window
        canvas.present();


        // --- Frame Timing ---
        let elapsed_time = frame_start_time.elapsed();
        if elapsed_time < TARGET_FRAME_DURATION {
            let sleep_duration = TARGET_FRAME_DURATION.saturating_sub(elapsed_time);
            // Use sleep for the majority of the wait time to yield CPU time
            if sleep_duration > Duration::from_millis(1) { // Avoid very short sleeps which can be inaccurate
                thread::sleep(sleep_duration.saturating_sub(Duration::from_millis(1)));
            }
            // Busy-wait or yield for the final moment for potentially higher accuracy
            while Instant::now() < frame_start_time + TARGET_FRAME_DURATION {
                // std::hint::spin_loop(); // Use if available and preferred for tight loops
                thread::yield_now(); // Fallback yield
            }
        }
        // Optional: Log if frame takes too long
        // else {
        //     println!("Frame took too long: {:?}", elapsed_time);
        // }
    } // End main_loop

    println!("Emulator stopped.");
    Ok(())
} // End main