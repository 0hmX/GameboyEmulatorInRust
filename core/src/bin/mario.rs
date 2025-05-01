use sdl2::pixels::Color;
use sdl2::rect::Rect; // Import Rect
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas; // Import Canvas
use sdl2::video::Window; // Import Window

use std::time::{Duration, Instant};
use std::thread;
use std::env;
use std::path::Path; // Import Path for better file handling

// Assuming these are in the lib.rs or similar of your crate
use gameboy_emulator::memory_bus::MemoryBus;
use gameboy_emulator::cpu::Cpu;
use gameboy_emulator::ppu::Ppu;

// --- Constants ---
const TARGET_FPS: u32 = 60;
const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000 / TARGET_FPS) as u64);

const CPU_FREQ_HZ: f64 = 4_194_304.0; // 4.194304 MHz
// Use floating point calculation for cycles per frame for better precision before casting
const CYCLES_PER_FRAME: u32 = (CPU_FREQ_HZ / TARGET_FPS as f64) as u32;

// Screen/Drawing Constants
const GB_WIDTH: u32 = 160;
const GB_HEIGHT: u32 = 144;
const SCALE_FACTOR: u32 = 4; // Increased scale slightly
const SCREEN_WIDTH: u32 = GB_WIDTH * SCALE_FACTOR;
const SCREEN_HEIGHT: u32 = GB_HEIGHT * SCALE_FACTOR;

// Define the Game Boy grayscale palette (a common one)
const PALETTE: [Color; 4] = [
    Color::RGB(0x9B, 0xBC, 0x0F), // Lightest Green/White
    Color::RGB(0x8B, 0xAC, 0x0F), // Light Gray/Green
    Color::RGB(0x30, 0x62, 0x30), // Dark Gray/Green
    Color::RGB(0x0F, 0x38, 0x0F), // Black/Darkest Green
];


/// Draws the Game Boy frame buffer to the SDL2 canvas.
/// `frame_buffer` should be a slice containing 160*144 bytes (shade 0-3).
fn draw(canvas: &mut Canvas<Window>, frame_buffer: &[u8]) {
    if frame_buffer.len() != (GB_WIDTH * GB_HEIGHT) as usize {
        canvas.set_draw_color(Color::RGB(255, 0, 0)); // Error color
        canvas.clear();
        eprintln!("Error: Frame buffer size mismatch! Expected {}, got {}", (GB_WIDTH * GB_HEIGHT), frame_buffer.len());
        canvas.present();
        return;
    }

    canvas.set_draw_color(PALETTE[0]); // Set background to lightest color
    canvas.clear();

    for y in 0..GB_HEIGHT {
        for x in 0..GB_WIDTH {
            let index = (y * GB_WIDTH + x) as usize;
            let shade_index = frame_buffer.get(index).copied().unwrap_or(0); // Default to 0 if out of bounds (shouldn't happen with check above)

            // Only draw if not the background color to potentially save fill calls
            if shade_index > 0 && shade_index < 4 {
                let color = PALETTE[shade_index as usize];
                canvas.set_draw_color(color);
                // Using fill_rect for scaling is simpler than drawing points
                let rect = Rect::new(
                    (x * SCALE_FACTOR) as i32,
                    (y * SCALE_FACTOR) as i32,
                    SCALE_FACTOR, // Width of scaled pixel
                    SCALE_FACTOR, // Height of scaled pixel
                );
                // Using unwrap here - if fill_rect fails, something is very wrong.
                canvas.fill_rect(rect).unwrap();
            } else if shade_index >= 4 {
                 // Draw error color for invalid shades (less likely now with get().copied())
                 canvas.set_draw_color(Color::RGB(255, 0, 255)); // Magenta error pixels
                  let rect = Rect::new(
                    (x * SCALE_FACTOR) as i32,
                    (y * SCALE_FACTOR) as i32,
                    SCALE_FACTOR,
                    SCALE_FACTOR,
                );
                canvas.fill_rect(rect).unwrap();
            }
            // If shade_index is 0, we don't need to draw, as the background is already cleared to PALETTE[0]
        }
    }

    canvas.present(); // Show the drawn frame
}


pub fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_path>", args[0]);
        std::process::exit(1);
    }
    let rom_path = Path::new(&args[1]); // Use Path for better file handling

    // --- SDL Initialization ---
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window_title = format!("Rust Game Boy Emu - {}", rom_path.file_name().unwrap_or_default().to_string_lossy());

    let window = video_subsystem
        .window(&window_title, SCREEN_WIDTH, SCREEN_HEIGHT) // Use scaled size
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas()
        .accelerated() // Use hardware acceleration
        //.present_vsync() // Keep VSync disabled for manual timing control
        .build()
        .map_err(|e| e.to_string())?;

    let mut event_pump = sdl_context.event_pump()?;

    // --- Emulator Initialization ---
    println!("Initializing memory bus...");
    let mut memory_bus = MemoryBus::new(); // MemoryBus owns the RAM, I/O regs etc.

    println!("Loading ROM: {}", rom_path.display());
    match std::fs::read(rom_path) {
        Ok(rom_data) => {
            memory_bus.load_rom(&rom_data);
            println!("ROM loaded successfully ({} bytes)", rom_data.len());
        }
        Err(e) => {
             eprintln!("Failed to load ROM '{}': {}", rom_path.display(), e);
             println!("Error: ROM loading failed. Exiting.");
             return Err(format!("Failed to load ROM: {}", e)); // Exit if ROM fails
         }
    }

    // --- Decide whether to skip boot ROM ---
    // Set to false if you have a boot ROM file and MemoryBus::new loads it
    // Set to true if you want to start directly in the game state
    let skip_boot_rom = true;

    println!("Initializing CPU (skip_boot_rom={})...", skip_boot_rom);
    // *** CHANGE: Cpu::new no longer takes memory_bus ***
    let mut cpu = Cpu::new(skip_boot_rom);

    // *** CHANGE: Initialize I/O registers if skipping boot ROM ***
    if skip_boot_rom {
        println!("Skipping boot ROM - initializing I/O registers...");
        Cpu::initialize_post_boot_io(&mut memory_bus); // Call the static helper
    } else {
        println!("Running boot ROM (ensure it's loaded in MemoryBus)...");
        // Make sure your MemoryBus::new() or other setup code loads the boot ROM
        // into the correct memory location (0x0000-0x00FF initially).
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
                    // memory_bus.handle_key_down(key); // Pass keycode to memory bus
                }
                Event::KeyUp { keycode: Some(key), repeat: false, .. } => {
                    // memory_bus.handle_key_up(key); // Pass keycode to memory bus
                }
                _ => {}
            }
        }

        // --- Emulate one frame ---
        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            // *** CHANGE: cpu.step() now requires &mut memory_bus ***
            // CPU executes instructions, returning cycles used. Reads/writes memory_bus.
            let executed_cycles = cpu.step(&mut memory_bus) as u32;

            // PPU processes based on the number of cycles the CPU took.
            // Updates state, requests interrupts via memory_bus (IF), renders to its buffer.
            // Needs memory_bus to read VRAM, OAM, LCDC, STAT, etc., and write IF, STAT, LY.
            ppu.step(executed_cycles, &mut memory_bus);

            // TODO: Tick other components like Timer, APU using executed_cycles and &mut memory_bus
            // Example: timer.step(executed_cycles, &mut memory_bus);
            // Example: apu.step(executed_cycles, &mut memory_bus);

            cycles_this_frame += executed_cycles;

            // Interrupt handling is done *within* cpu.step(), checking IF/IE from memory_bus.
        }

        // --- Drawing ---
        // Get the frame buffer generated by the PPU during the emulation loop
        let frame_to_draw = ppu.get_frame_buffer();
        draw(&mut canvas, frame_to_draw);


        // --- Frame Timing ---
        let elapsed_time = frame_start_time.elapsed();
        if elapsed_time < TARGET_FRAME_DURATION {
            // Use sleep for the majority of the wait time to yield CPU
            let sleep_duration = TARGET_FRAME_DURATION.saturating_sub(elapsed_time);
            // Only sleep if the duration is significant enough to avoid very short sleeps
            if sleep_duration > Duration::from_millis(1) {
                thread::sleep(sleep_duration.saturating_sub(Duration::from_millis(1))); // Sleep for most of the remaining time
            }
             // Busy-wait for the final moment for potentially higher accuracy
            while Instant::now() < frame_start_time + TARGET_FRAME_DURATION {
                 // std::hint::spin_loop(); // Modern alternative to yield_now in tight loops
                 thread::yield_now(); // Fallback if spin_loop is not preferred/available
             }
        } else {
            // Frame took too long, might log this if it happens frequently
            // println!("Frame took too long: {:?}", elapsed_time);
        }

        // Update window title with FPS (optional, can add overhead)
        // let actual_fps = 1.0 / frame_start_time.elapsed().as_secs_f64();
        // let title = format!("{} - FPS: {:.2}", rom_path.file_name().unwrap_or_default().to_string_lossy(), actual_fps);
        // canvas.window_mut().set_title(&title).ok();
    }

    Ok(())
}

// --- Add basic key mapping inside MemoryBus or a dedicated Input module ---
// Example placeholder functions needed in MemoryBus:
/*
impl MemoryBus {
    pub fn handle_key_down(&mut self, key: Keycode) {
        // Map SDL key to GB key and update self.joypad_state
        // Remember to request Joypad interrupt (bit 4 in IF) if appropriate
        println!("Key Down: {:?}", key); // Placeholder
    }

    pub fn handle_key_up(&mut self, key: Keycode) {
        // Map SDL key to GB key and update self.joypad_state
        println!("Key Up: {:?}", key); // Placeholder
    }
}
*/