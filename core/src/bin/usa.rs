use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::time::{Duration, Instant};
use std::thread;

use gameboy_emulator::memory_bus::MemoryBus;
use gameboy_emulator::cpu::Cpu;

use std::env;

const TARGET_FPS: u32 = 60; // Game Boy runs at ~59.7 FPS, 60 is a common target
const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000 / TARGET_FPS) as u64);

const CPU_FREQ_HZ: f64 = 4.194304 * 1_000_000.0; // 4.194304 MHz
const CYCLES_PER_FRAME: u32 = (CPU_FREQ_HZ / TARGET_FPS as f64) as u32;


static WIDTH: u32 = 160; // Game Boy screen width
static HIGHT: u32 = 144; // Game Boy screen height

fn draw(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    // Placeholder drawing - remove when PPU is integrated
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.draw_point(Point::new(80, 72)).expect("Could not draw point");

    canvas.present(); // Show the drawn frame
}

pub fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_path>", args[0]);
        std::process::exit(1);
    }
    let rom_path = &args[1];

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Rust Game Boy Emu (Basic)", WIDTH, HIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas()
        .accelerated()
        .present_vsync()
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
            let dummy_rom = vec![0x00; 0x8000];
            memory_bus.load_rom(&dummy_rom);
            println!("Warning: Loaded dummy ROM instead");
        }
    }

    println!("Initializing CPU...");
    let mut cpu = Cpu::new(&mut memory_bus, false); // Assuming no boot ROM support yet

    println!("Starting main loop...");
    'main_loop: loop {
        let frame_start_time = Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Exiting main loop.");
                    break 'main_loop;
                }
                _ => {}
            }
        }

        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            let executed_cycles = cpu.step();
            cycles_this_frame += executed_cycles as u32;
        }

        draw(&mut canvas);

        let elapsed_time = frame_start_time.elapsed();
        if elapsed_time < TARGET_FRAME_DURATION {
            thread::sleep(TARGET_FRAME_DURATION.saturating_sub(elapsed_time));
        }
    }

    Ok(())
}