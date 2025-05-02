use sdl2::ttf::Font; // Import Keycode
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

    // --- Setup SDL Context ---
    let mut sdl_context = sdl_setup::init_sdl(&window_title)?;

    // --- Load Font ---
    println!("Loading font: {}...", constants::FONT_PATH);
    let font_path = Path::new(constants::FONT_PATH);
    if !font_path.exists() {
        return Err(format!("Font file not found: {}", constants::FONT_PATH));
    }
    let font: Font = sdl_context.ttf_context
        .load_font(font_path, constants::DEBUG_FONT_SIZE)?;
    println!("Font loaded successfully.");

    // --- Setup Emulator ---
    let mut emulator = Emulator::new(rom_path, true)?;

    // --- Pre-calculate drawing coordinates ---
    let gb_screen_x = 0;
    let gb_screen_y = 0;
    let disasm_pane_x = (constants::GB_SCREEN_WIDTH + constants::PADDING) as i32;
    let disasm_pane_y = 0;
    let far_right_pane_x = disasm_pane_x + constants::DISASM_AREA_WIDTH as i32 + constants::PADDING as i32;
    let vram_view_y = 0;
    let input_view_y = constants::VRAM_VIEW_HEIGHT as i32 + constants::PADDING as i32; // Adjusted based on constants layout

    // --- Added: State for step/toggle key presses to prevent rapid multi-triggering ---
    let mut p_key_pressed_last_frame = false;
    let mut n_key_pressed_last_frame = false;

    // --- Main Loop ---
    println!("Starting main loop...");
    'main_loop: loop {
        let frame_start_time = Instant::now();

        // --- 1. Handle Input (Original signature: only returns quit signal) ---
        // This will handle standard emulator inputs (A, B, Start, Select, D-Pad, Quit)
        if input::handle_input(&mut sdl_context.event_pump, &mut emulator.memory_bus) {
            break 'main_loop;
        }

        // --- Added: Check for Stepping Control Keys ---
        let keyboard_state = sdl_context.event_pump.keyboard_state();
        let p_key_currently_pressed = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::P);
        let n_key_currently_pressed = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::N);

        let mut step_executed_this_iteration = false;

        // Toggle Stepping on P key *press* (rising edge)
        if p_key_currently_pressed && !p_key_pressed_last_frame {
            emulator.toggle_stepping();
        }

        // Execute Step on N key *press* (rising edge) *if* in stepping mode
        if emulator.stepping && n_key_currently_pressed && !n_key_pressed_last_frame {
            println!("Executing one step..."); // Debug message
            if let Err(e) = emulator.step_instruction() {
                eprintln!("Emulator Step Error: {}", e);
                // Decide if stepping error halts the program
                // break 'main_loop;
            }
            step_executed_this_iteration = true; // Mark that a step happened
        }

        // Update last frame state for keys
        p_key_pressed_last_frame = p_key_currently_pressed;
        n_key_pressed_last_frame = n_key_currently_pressed;
        // --- End Added ---


        // --- 2. Emulate One Frame (Conditional) ---
        // Only run full frame if not in stepping mode
        if !emulator.stepping {
             if let Err(e) = emulator.run_frame() {
                eprintln!("Emulator Error: {}", e);
                break 'main_loop;
             }
        }
        // Note: Single step execution is handled above based on 'N' key press

        // --- 3. Update Debug Views ---
        // Original logic: always update. We'll keep this for simplicity,
        // although it could be optimized to only update when state changes.
        // If optimizing: update if !emulator.stepping || step_executed_this_iteration
        emulator.ppu.update_vram_debug_buffer(&emulator.memory_bus);


        // --- 4. Drawing ---
        // Original logic: Draw every frame, which is correct.
        sdl_context.canvas.set_draw_color(constants::DEBUG_BACKGROUND_COLOR); // Use consistent background
        sdl_context.canvas.clear();

        // Draw GB Screen
        if let Err(e) = drawing::draw_gb_screen(
            &mut sdl_context.canvas,
            emulator.ppu.get_frame_buffer(),
            gb_screen_x,
            gb_screen_y,
        ) {
             eprintln!("Error drawing GB screen: {}", e);
        }

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
        if let Err(e) = drawing::draw_vram_debug(
            &mut sdl_context.canvas,
            emulator.ppu.get_vram_debug_buffer(),
            far_right_pane_x,
            vram_view_y,
        ) {
             eprintln!("Error drawing VRAM: {}", e);
        }

        // Draw Input View
         // Get the current state directly from the joypad struct within the memory bus
        if let Err(e) = drawing::draw_input_debug(
            &mut sdl_context.canvas,
            &emulator.memory_bus.joypad.get_state(), // Get fresh state
            far_right_pane_x,
            input_view_y,
        ) {
             eprintln!("Error drawing Input: {}", e);
        }

        sdl_context.canvas.present();

        // --- 5. Frame Timing (Conditional) ---
        // Original timing logic, but only apply if NOT stepping
        if !emulator.stepping {
            let elapsed_time = frame_start_time.elapsed();
            if elapsed_time < constants::TARGET_FRAME_DURATION {
                let sleep_duration = constants::TARGET_FRAME_DURATION.saturating_sub(elapsed_time);
                // Using original sleep/yield logic:
                if sleep_duration > Duration::from_millis(1) {
                    thread::sleep(sleep_duration.saturating_sub(Duration::from_millis(1)));
                }
                while Instant::now() < frame_start_time + constants::TARGET_FRAME_DURATION {
                    thread::yield_now();
                }
            }
        } else {
            // Added: If stepping, maybe sleep briefly to avoid maxing CPU when idle
             if !step_executed_this_iteration { // Avoid sleeping right after stepping
                 thread::sleep(Duration::from_millis(5)); // Reduce CPU usage while paused
             }
        }

    } // End 'main_loop

    println!("Emulator stopped.");
    Ok(())
}