use sdl2::{pixels::Color, ttf::Font}; // Need to import Font here now
use std::{env, path::Path, thread, time::{Duration, Instant}};

// Declare modules located within the src/app/ directory
mod constants;
mod sdl_setup;
mod drawing;
mod input;
mod emulator;

use emulator::Emulator;

fn main() -> Result<(), String> {
    // --- Argument Parsing ---
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_path>", args[0]);
        std::process::exit(1);
    }
    let rom_path = Path::new(&args[1]);
    let rom_filename = rom_path.file_name().unwrap_or_default().to_string_lossy();
    let window_title = format!("Rust GB Emu - {}", rom_filename);

    // --- Setup SDL Context (without font) ---
    let mut sdl_context = sdl_setup::init_sdl(&window_title)?;

    // --- Load Font using the TTF context from SdlContext ---
    println!("Loading font: {}...", constants::FONT_PATH);
    let font_path = Path::new(constants::FONT_PATH);
    if !font_path.exists() {
        return Err(format!("Font file not found: {}", constants::FONT_PATH));
    }
    // The font borrows from sdl_context.ttf_context. Both live in `main` scope.
    let font: Font = sdl_context.ttf_context
        .load_font(font_path, constants::DEBUG_FONT_SIZE)?;
    println!("Font loaded successfully.");

    // --- Setup Emulator ---
    let mut emulator = Emulator::new(rom_path, true)?;

    // --- Pre-calculate drawing coordinates ---
    // ... (coordinate calculations remain the same) ...
    let vram_view_width = constants::VRAM_VIEW_WIDTH;
    let vram_view_height = constants::VRAM_VIEW_HEIGHT;
    let gb_screen_x = 0;
    let gb_screen_y = 0;
    let disasm_pane_x = (constants::GB_SCREEN_WIDTH + constants::PADDING) as i32;
    let disasm_pane_y = 0;
    let far_right_pane_x = disasm_pane_x + constants::DISASM_AREA_WIDTH as i32 + constants::PADDING as i32;
    let vram_view_y = 0;
    let input_view_y = vram_view_y + vram_view_height as i32 + constants::PADDING as i32;


    // --- Main Loop ---
    println!("Starting main loop...");
    'main_loop: loop {
        let frame_start_time = Instant::now();

        // --- 1. Handle Input ---
        if input::handle_input(&mut sdl_context.event_pump, &mut emulator.memory_bus) {
            break 'main_loop;
        }

        // --- 2. Emulate One Frame ---
        if let Err(e) = emulator.run_frame() {
            eprintln!("Emulator Error: {}", e);
            break 'main_loop;
        }

        // --- 3. Update Debug Views ---
        emulator.ppu.update_vram_debug_buffer(&emulator.memory_bus);

        // --- 4. Drawing ---
        sdl_context.canvas.set_draw_color(Color::RGB(20, 20, 20));
        sdl_context.canvas.clear();

        // Draw GB Screen
        drawing::draw_gb_screen(
            &mut sdl_context.canvas,
            emulator.ppu.get_frame_buffer(),
            gb_screen_x,
            gb_screen_y,
        );

        // Draw Disassembly - Pass the locally loaded font
        if let Err(e) = drawing::draw_disassembly_debug(
            &mut sdl_context.canvas,
            &sdl_context.texture_creator,
            &font, // Pass the font loaded in main
            &emulator.cpu,
            &emulator.memory_bus,
            disasm_pane_x,
            disasm_pane_y,
        ) {
            eprintln!("Error drawing disassembly: {}", e);
        }

        // Draw VRAM View
        drawing::draw_vram_debug(
            &mut sdl_context.canvas,
            emulator.ppu.get_vram_debug_buffer(),
            far_right_pane_x,
            vram_view_y,
        );

        // Draw Input View
        drawing::draw_input_debug(
            &mut sdl_context.canvas,
            &emulator.memory_bus.joypad.get_state(),
            far_right_pane_x,
            input_view_y,
        );

        sdl_context.canvas.present();

        // --- 5. Frame Timing ---
        let elapsed_time = frame_start_time.elapsed();
        if elapsed_time < constants::TARGET_FRAME_DURATION {
            let sleep_duration = constants::TARGET_FRAME_DURATION.saturating_sub(elapsed_time);
            if sleep_duration > Duration::from_millis(1) {
                thread::sleep(sleep_duration.saturating_sub(Duration::from_millis(1)));
            }
            while Instant::now() < frame_start_time + constants::TARGET_FRAME_DURATION {
                thread::yield_now();
            }
        }
    } // End 'main_loop

    println!("Emulator stopped.");
    Ok(())
}