use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect}; // Import Rect
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas; // Import Canvas
use sdl2::video::Window; // Import Window

use std::time::{Duration, Instant};
use std::thread;
use std::env;

use gameboy_emulator::memory_bus::MemoryBus;
use gameboy_emulator::cpu::Cpu;
use gameboy_emulator::ppu::Ppu;

// --- Constants ---
const TARGET_FPS: u32 = 60;
const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000 / TARGET_FPS) as u64);

const CPU_FREQ_HZ: f64 = 4.194304 * 1_000_000.0;
const CYCLES_PER_FRAME: u32 = (CPU_FREQ_HZ / TARGET_FPS as f64) as u32;

// Screen/Drawing Constants
const GB_WIDTH: u32 = 160;
const GB_HEIGHT: u32 = 144;
const SCALE_FACTOR: u32 = 3; // Adjust scaling as needed
const SCREEN_WIDTH: u32 = GB_WIDTH * SCALE_FACTOR;
const SCREEN_HEIGHT: u32 = GB_HEIGHT * SCALE_FACTOR;

// Define the Game Boy grayscale palette
const PALETTE: [Color; 4] = [
    Color::RGB(224, 248, 208), // 0: Lightest Green/White
    Color::RGB(136, 192, 112), // 1: Light Gray/Green
    Color::RGB(52, 104, 86),   // 2: Dark Gray/Green
    Color::RGB(8, 24, 32),     // 3: Black/Darkest Green
];


/// Draws the Game Boy frame buffer to the SDL2 canvas.
/// `frame_buffer` should be a slice containing 160*144 bytes (shade 0-3).
fn draw(canvas: &mut Canvas<Window>, frame_buffer: &[u8]) {
    if frame_buffer.len() != (GB_WIDTH * GB_HEIGHT) as usize {
        canvas.set_draw_color(Color::RGB(255, 0, 0)); // Error color
        canvas.clear();
        // Optionally add logging or a visible error message on screen
        canvas.present();
        return;
    }

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    for y in 0..GB_HEIGHT {
        for x in 0..GB_WIDTH {
            let index = (y * GB_WIDTH + x) as usize;
            let shade_index = frame_buffer[index];

            if shade_index < 4 {
                let color = PALETTE[shade_index as usize];
                canvas.set_draw_color(color);
                let rect = Rect::new(
                    (x * SCALE_FACTOR) as i32,
                    (y * SCALE_FACTOR) as i32,
                    SCALE_FACTOR,
                    SCALE_FACTOR,
                );
                canvas.fill_rect(rect).expect("Could not fill rect");
            } else {
                 // Draw error color for invalid shades
                 canvas.set_draw_color(Color::RGB(255, 0, 255)); // Magenta error pixels
                  let rect = Rect::new(
                    (x * SCALE_FACTOR) as i32,
                    (y * SCALE_FACTOR) as i32,
                    SCALE_FACTOR,
                    SCALE_FACTOR,
                );
                canvas.fill_rect(rect).expect("Could not fill rect");
            }
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
    let rom_path = &args[1];

    // --- SDL Initialization ---
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Rust Game Boy Emu", SCREEN_WIDTH, SCREEN_HEIGHT) // Use scaled size
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas()
        .accelerated() // Use hardware acceleration
        //.present_vsync() // VSync can sometimes interfere with precise timing, test performance
        .build()
        .map_err(|e| e.to_string())?;

    let mut event_pump = sdl_context.event_pump()?;

    // --- Emulator Initialization ---
    println!("Initializing memory bus...");
    let mut memory_bus = MemoryBus::new();

    println!("Loading ROM: {}", rom_path);
    match std::fs::read(rom_path) {
        Ok(rom_data) => {
            memory_bus.load_rom(&rom_data);
            println!("ROM loaded successfully ({} bytes)", rom_data.len());
        }
        Err(e) => {
             eprintln!("Failed to load ROM '{}': {}", rom_path, e);
             // Use a slightly more interesting dummy ROM for visual testing
             let mut dummy_rom = vec![0u8; 0x8000];
             // Add a JP 0150 loop to prevent running off into nowhere too quickly
             dummy_rom[0x100] = 0xC3; // JP nn
             dummy_rom[0x101] = 0x50; // Low byte of 0150
             dummy_rom[0x102] = 0x01; // High byte of 0150
             dummy_rom[0x150] = 0xC3; // JP nn
             dummy_rom[0x151] = 0x50; // Low byte of 0150
             dummy_rom[0x152] = 0x01; // High byte of 0150
             memory_bus.load_rom(&dummy_rom);
             println!("Warning: Loaded dummy ROM instead");
         }
    }


    println!("Initializing CPU...");
    // Pass true to skip_boot_rom if you want post-boot register values
    // You might need to adjust this based on whether your memory_bus/ppu handle boot rom behavior
    let skip_boot_rom = true;
    let mut cpu = Cpu::new(&mut memory_bus, skip_boot_rom);

    println!("Initializing PPU...");
    let mut ppu = Ppu::new(); // Instantiate the PPU

    println!("Starting main loop...");
    'main_loop: loop {
        let frame_start_time = Instant::now();

        // --- Handle Input ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Exiting main loop.");
                    break 'main_loop;
                }
                // TODO: Add KeyDown/KeyUp handling to update Joypad state in MemoryBus (0xFF00)
                // Event::KeyDown { keycode: Some(key), .. } => { /* memory_bus.key_down(key); */ }
                // Event::KeyUp { keycode: Some(key), .. } => { /* memory_bus.key_up(key); */ }
                _ => {}
            }
        }

        // --- Emulate one frame ---
        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            // CPU executes instructions, potentially modifying memory (including PPU regs, IF)
            // The number of cycles depends on the instruction executed.
            let executed_cycles = cpu.step() as u32;

            // PPU processes based on the number of cycles the CPU took.
            // It updates its internal state (mode, LY), potentially modifies memory (STAT, LY),
            // requests interrupts (IF), and renders pixels to its internal buffer.
            ppu.step(executed_cycles, &mut memory_bus);

            // TODO: Tick other components like Timer, APU using executed_cycles

            cycles_this_frame += executed_cycles;

            // Note: Interrupt handling is done *within* cpu.step() at the beginning of the step.
            // The PPU requesting an interrupt by setting IF will be checked by the CPU on its next step.
        }

        // --- Drawing ---
        // Get the frame buffer generated by the PPU during the emulation loop
        let frame_to_draw = ppu.get_frame_buffer();
        draw(&mut canvas, frame_to_draw);


        // --- Frame Timing ---
        let elapsed_time = frame_start_time.elapsed();
        if elapsed_time < TARGET_FRAME_DURATION {
            // Busy-wait for potentially higher accuracy than thread::sleep
            while Instant::now() < frame_start_time + TARGET_FRAME_DURATION {
                 thread::yield_now(); // Yield CPU time slice while waiting
                 // std::hint::spin_loop(); // Consider using spin_loop hint if available/needed
             }
        }
    }

    Ok(())
}