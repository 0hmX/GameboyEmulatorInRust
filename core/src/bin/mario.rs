use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::render::{Canvas, TextureCreator}; // Import TextureCreator
use sdl2::video::{Window, WindowContext}; // Import WindowContext

// --- TTF Imports ---
use sdl2::ttf::{self, Font}; // Import ttf and Font

use std::time::{Duration, Instant};
use std::thread;
use std::env;
use std::path::Path;
use std::cmp;
use std::cell::RefCell; // Needed for shared mutable access if MemoryBus holds APU
use std::rc::Rc;        // Needed for shared mutable access if MemoryBus holds APU

// --- Emulator Core Imports ---
use gameboy_emulator::memory_bus::{MemoryBus, JoypadState};
use gameboy_emulator::cpu::Cpu;
use gameboy_emulator::ppu::{Ppu, VRAM_DEBUG_WIDTH, VRAM_DEBUG_HEIGHT};
use gameboy_emulator::apu::Apu; 

// --- Timing Constants ---
const TARGET_FPS: u32 = 60;
const TARGET_FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000 / TARGET_FPS) as u64);

// --- CPU/Emulator Constants ---
const CPU_FREQ_HZ: f64 = 4_194_304.0;
const CYCLES_PER_FRAME: u32 = (CPU_FREQ_HZ / TARGET_FPS as f64) as u32;

// --- Screen/Drawing Constants ---
const GB_WIDTH: u32 = 160;
const GB_HEIGHT: u32 = 144;
const GB_SCALE_FACTOR: u32 = 3; // Adjusted scale slightly for more debug room
const VRAM_DEBUG_SCALE_FACTOR: u32 = 2;
const PADDING: u32 = 10;

// --- Input Debug Visualizer Constants ---
const DEBUG_INPUT_BOX_SIZE: u32 = 15; // Smaller boxes
const DEBUG_INPUT_PADDING: u32 = 4;
const DEBUG_INPUT_PRESSED_COLOR: Color = Color::RGB(50, 205, 50);
const DEBUG_INPUT_RELEASED_COLOR: Color = Color::RGB(70, 70, 70);

// --- Disassembly Debug Constants ---
const FONT_PATH: &str = "/home/ankan/GameBoy/core/src/bin/Roboto-Regular.ttf"; // IMPORTANT: Change this path!
const DEBUG_FONT_SIZE: u16 = 14;
const DISASM_LINES_BEFORE: usize = 5; // Number of lines before PC
const DISASM_LINES_AFTER: usize = 10; // Number of lines after (and including) PC
const DISASM_LINE_HEIGHT: u32 = (DEBUG_FONT_SIZE + 2) as u32; // Font size + padding
const DISASM_AREA_WIDTH: u32 = 300; // Estimated width needed
const DISASM_AREA_HEIGHT: u32 = DISASM_LINE_HEIGHT * (DISASM_LINES_BEFORE + DISASM_LINES_AFTER + 1) as u32;
const DEBUG_PC_COLOR: Color = Color::RGB(255, 255, 0); // Yellow for current PC line
const DEBUG_TEXT_COLOR: Color = Color::RGB(220, 220, 220); // Light Gray for other lines
const DEBUG_BACKGROUND_COLOR: Color = Color::RGB(30, 30, 30); // Darker background for text

// --- Calculate Constant Dimensions ---
const GB_SCREEN_WIDTH: u32 = GB_WIDTH * GB_SCALE_FACTOR;
const GB_SCREEN_HEIGHT: u32 = GB_HEIGHT * GB_SCALE_FACTOR;
const VRAM_VIEW_WIDTH: u32 = VRAM_DEBUG_WIDTH as u32 * VRAM_DEBUG_SCALE_FACTOR;
const VRAM_VIEW_HEIGHT: u32 = VRAM_DEBUG_HEIGHT as u32 * VRAM_DEBUG_SCALE_FACTOR;
const DPAD_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 3 + DEBUG_INPUT_PADDING * 2;
const DPAD_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 3 + DEBUG_INPUT_PADDING * 2;
const ACTION_AREA_WIDTH: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1;
const ACTION_AREA_HEIGHT: u32 = DEBUG_INPUT_BOX_SIZE * 2 + DEBUG_INPUT_PADDING * 1;
const INPUT_DEBUG_AREA_WIDTH: u32 = DPAD_AREA_WIDTH + PADDING + ACTION_AREA_WIDTH;

// --- Palettes ---
const PALETTE: [Color; 4] = [
    Color::RGB(0x9B, 0xBC, 0x0F), Color::RGB(0x8B, 0xAC, 0x0F),
    Color::RGB(0x30, 0x62, 0x30), Color::RGB(0x0F, 0x38, 0x0F),
];
const DEBUG_PALETTE: [Color; 4] = [
    Color::RGB(0xFF, 0xFF, 0xFF), Color::RGB(0xAA, 0xAA, 0xAA),
    Color::RGB(0x55, 0x55, 0x55), Color::RGB(0x00, 0x00, 0x00),
];

pub type CpuResult<T> = Result<T, String>;

// --- Drawing Helper Functions ---

/// Renders text using SDL_ttf.
fn render_text(
    canvas: &mut Canvas<Window>,
    texture_creator: &TextureCreator<WindowContext>,
    font: &Font,
    text: &str,
    x: i32,
    y: i32,
    color: Color,
) -> Result<(), String> {
    if text.is_empty() {
        return Ok(()); // Nothing to render
    }
    let surface = font.render(text)
        .blended(color) // Use blended for nice anti-aliasing
        .map_err(|e| e.to_string())?;
    let texture = texture_creator.create_texture_from_surface(&surface)
        .map_err(|e| e.to_string())?;
    let text_rect = Rect::new(x, y, surface.width(), surface.height());
    canvas.copy(&texture, None, Some(text_rect))?;
    Ok(())
}


fn draw_gb_screen(canvas: &mut Canvas<Window>, frame_buffer: &[u8], target_x: i32, target_y: i32) {
    if frame_buffer.len() != (GB_WIDTH * GB_HEIGHT) as usize {
        eprintln!("Error: Frame buffer size mismatch! Expected {}, got {}", (GB_WIDTH * GB_HEIGHT), frame_buffer.len());
        return;
    }
    for y in 0..GB_HEIGHT {
        for x in 0..GB_WIDTH {
            let index = (y * GB_WIDTH + x) as usize;
            let shade_index = frame_buffer.get(index).copied().unwrap_or(0) % 4;
            let color = PALETTE[shade_index as usize];

            canvas.set_draw_color(color);
            let rect = Rect::new(
                target_x + (x * GB_SCALE_FACTOR) as i32,
                target_y + (y * GB_SCALE_FACTOR) as i32,
                GB_SCALE_FACTOR,
                GB_SCALE_FACTOR,
            );
            canvas.fill_rect(rect).unwrap();
        }
    }
}

fn draw_vram_debug(canvas: &mut Canvas<Window>, vram_buffer: &[u8], target_x: i32, target_y: i32) {
    let expected_len = VRAM_DEBUG_WIDTH * VRAM_DEBUG_HEIGHT;
     if vram_buffer.len() != expected_len {
         eprintln!("Error: VRAM debug buffer size mismatch! Expected {}, got {}", expected_len, vram_buffer.len());
         return;
     }
    for y in 0..VRAM_DEBUG_HEIGHT {
        for x in 0..VRAM_DEBUG_WIDTH {
            let index = y * VRAM_DEBUG_WIDTH + x;
            let shade_index = vram_buffer.get(index).copied().unwrap_or(0) % 4;
            let color = DEBUG_PALETTE[shade_index as usize];

            canvas.set_draw_color(color);
            let rect = Rect::new(
                target_x + (x as u32 * VRAM_DEBUG_SCALE_FACTOR) as i32,
                target_y + (y as u32 * VRAM_DEBUG_SCALE_FACTOR) as i32,
                VRAM_DEBUG_SCALE_FACTOR,
                VRAM_DEBUG_SCALE_FACTOR,
            );
             canvas.fill_rect(rect).unwrap();
        }
    }
}

fn draw_input_debug(canvas: &mut Canvas<Window>, joypad_state: &JoypadState, target_x: i32, target_y: i32) {
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

    let pad_step = (DEBUG_INPUT_BOX_SIZE + DEBUG_INPUT_PADDING) as i32;
    let dpad_center_x = pad_step;
    let dpad_center_y = pad_step;

    draw_indicator(joypad_state.up, dpad_center_x, dpad_center_y - pad_step);
    draw_indicator(joypad_state.down, dpad_center_x, dpad_center_y + pad_step);
    draw_indicator(joypad_state.left, dpad_center_x - pad_step, dpad_center_y);
    draw_indicator(joypad_state.right, dpad_center_x + pad_step, dpad_center_y);

    let action_start_x = DPAD_AREA_WIDTH as i32 + PADDING as i32;
    let action_y1 = 0;
    let action_y2 = pad_step;

    draw_indicator(joypad_state.b, action_start_x, action_y1);
    draw_indicator(joypad_state.a, action_start_x + pad_step, action_y1);
    draw_indicator(joypad_state.select, action_start_x, action_y2);
    draw_indicator(joypad_state.start, action_start_x + pad_step, action_y2);
}

/// Draws the disassembly debug view.
/// Assumes `cpu.disassemble_instruction(addr, bus)` exists and is public.
fn draw_disassembly_debug(
    canvas: &mut Canvas<Window>,
    texture_creator: &TextureCreator<WindowContext>,
    font: &Font,
    cpu: &Cpu,
    bus: &MemoryBus,
    target_x: i32,
    target_y: i32,
    lines_before: usize,
    lines_after: usize,
) -> Result<(), String> {
    // Draw a background rectangle for the disassembly area
    canvas.set_draw_color(DEBUG_BACKGROUND_COLOR);
    let bg_rect = Rect::new(target_x, target_y, DISASM_AREA_WIDTH, DISASM_AREA_HEIGHT);
    canvas.fill_rect(bg_rect).map_err(|e| e.to_string())?;


    let current_pc = cpu.pc;
    let total_lines = lines_before + 1 + lines_after;
    let mut instructions: Vec<(u16, String)> = Vec::with_capacity(total_lines);

    // --- Disassemble Forwards (including PC) ---
    let mut current_addr = current_pc; // Start disassembling from PC
    for _ in 0..=lines_after {
        // *** This relies on the public disassemble_instruction method ***
        // *** Ensure MemoryBus provides necessary read access for disassembly ***
        let (disasm_text, instr_len) = cpu.disassemble_instruction(current_addr, bus);
        instructions.push((current_addr, disasm_text));
        // Advance address, handling potential overflow (though unlikely in practice here)
        current_addr = current_addr.wrapping_add(instr_len as u16);
        if instr_len == 0 { break; } // Avoid infinite loop if disassembler returns 0 len
    }

    // --- Disassemble Backwards (Approximate) ---
    // This is tricky because instruction lengths vary. We find the PC line
    // and try to render `lines_before` lines before it.
    // A more robust way would involve iterating forward from an earlier address,
    // but this simpler approach works visually for many cases.
    let mut start_addr = current_pc;
    for _ in 0..lines_before {
        // Estimate previous instruction was 1-3 bytes back. Try 3 bytes back first.
        let mut found_prev = false;
        for offset_guess in (1..=3).rev() { // Try 3, 2, 1 bytes back
            let prev_addr_guess = start_addr.wrapping_sub(offset_guess);
            // Check if disassembling from guess lands us back at start_addr
            let (_, len_guess) = cpu.disassemble_instruction(prev_addr_guess, bus);
             if len_guess == (offset_guess as u8) {
                 // Found a likely previous instruction boundary
                 start_addr = prev_addr_guess;
                 let (disasm_text, _) = cpu.disassemble_instruction(start_addr, bus);
                 // Prepend to our list
                 instructions.insert(0, (start_addr, disasm_text));
                 found_prev = true;
                 break;
             }
        }
         if !found_prev {
             // Couldn't reliably step back, stop trying
             break;
         }
    }

    // --- Render the collected lines ---
    let mut current_y = target_y;
    // Find the index of the PC instruction in our potentially adjusted list
    let pc_index_maybe = instructions.iter().position(|(addr, _)| *addr == current_pc);

    let num_lines_to_render = lines_before + 1 + lines_after;
    let mut rendered_count = 0;

    // Determine the starting index for rendering
    let start_render_idx = if let Some(pc_index) = pc_index_maybe {
        pc_index.saturating_sub(lines_before)
    } else {
        // If PC wasn't found (maybe edge case during backwards search?),
        // try to render from beginning up to max lines.
        0
    };


    for (idx, (addr, text)) in instructions.iter().enumerate().skip(start_render_idx) {
        if rendered_count >= num_lines_to_render {
            break;
        }

        let display_text = format!("{:04X}: {}", addr, text);
        let color = if *addr == current_pc {
            DEBUG_PC_COLOR
        } else {
            DEBUG_TEXT_COLOR
        };

        render_text(canvas, texture_creator, font, &display_text, target_x + 5, current_y, color)?; // Add small X padding

        current_y += DISASM_LINE_HEIGHT as i32;
        rendered_count += 1;
    }

    Ok(())
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
    // TODO: Initialize SDL Audio Subsystem if generating sound

    // --- TTF Initialization ---
    println!("Initializing SDL2_ttf...");
    let ttf_context = ttf::init().map_err(|e| e.to_string())?;

    // --- Load Font ---
    println!("Loading font: {}...", FONT_PATH);
    let font_path = Path::new(FONT_PATH);
    if !font_path.exists() {
         eprintln!("FATAL: Font file not found at '{}'. Please ensure the path is correct and the file exists.", FONT_PATH);
         return Err(format!("Font file not found: {}", FONT_PATH));
    }
    let font = ttf_context.load_font(font_path, DEBUG_FONT_SIZE)?;
    println!("Font loaded successfully.");


    let window_title = format!("Rust GB Emu - {}", rom_path.file_name().unwrap_or_default().to_string_lossy());

    // --- Calculate Window Dimensions at Runtime ---
    let input_debug_area_height: u32 = cmp::max(DPAD_AREA_HEIGHT, ACTION_AREA_HEIGHT);
    let mid_pane_height = VRAM_VIEW_HEIGHT + PADDING + input_debug_area_height;
    let right_pane_height = mid_pane_height + PADDING + DISASM_AREA_HEIGHT;
    let right_pane_width = cmp::max(cmp::max(VRAM_VIEW_WIDTH, INPUT_DEBUG_AREA_WIDTH), DISASM_AREA_WIDTH);
    let total_window_height: u32 = cmp::max(GB_SCREEN_HEIGHT, right_pane_height);
    let total_window_width: u32 = GB_SCREEN_WIDTH + PADDING + right_pane_width;

    println!("Creating window ({}x{})...", total_window_width, total_window_height);
    let window = video_subsystem
        .window(&window_title, total_window_width, total_window_height)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    println!("Creating accelerated canvas...");
    let mut canvas = window.into_canvas()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    println!("Initializing event pump...");
    let mut event_pump = sdl_context.event_pump()?;

    // --- Emulator Initialization ---
    println!("Initializing APU...");
    let apu = Apu::new();
    // *** Modify MemoryBus::new to accept APU if needed ***
    // let apu_ref = Rc::new(RefCell::new(apu));
    // let mut memory_bus = MemoryBus::new(apu_ref.clone());
    println!("Initializing memory bus...");
    let mut memory_bus = MemoryBus::new(); // <-- Placeholder - MUST BE MODIFIED if APU is integrated via bus

    println!("Loading ROM: {}", rom_path.display());
    match std::fs::read(rom_path) {
        Ok(rom_data) => { // Use a block to print size before borrow/move
            let rom_size = rom_data.len();
            memory_bus.load_rom(&rom_data);
            println!("ROM loaded successfully ({} bytes)", rom_size); // Moved success message here
        }
        Err(e) => return Err(format!("Failed to load ROM '{}': {}", rom_path.display(), e)),
    }
    println!("ROM loaded successfully.");

    let skip_boot_rom = true;
    println!("Initializing CPU (skip_boot_rom={})...", skip_boot_rom);
    let mut cpu = Cpu::new(skip_boot_rom);

    if skip_boot_rom {
        println!("Skipping boot ROM - initializing I/O registers post-boot...");
        Cpu::initialize_post_boot_io(&mut memory_bus);
    }

    println!("Initializing PPU...");
    let mut ppu = Ppu::new();
    // Make APU mutable if owned by main loop
    let mut apu = apu;

    println!("Starting main loop...");
    'main_loop: loop {
        let frame_start_time = Instant::now();

        // --- 1. Handle Input ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Exit requested.");
                    break 'main_loop;
                }
                Event::KeyDown { keycode: Some(key), repeat: false, .. } => memory_bus.key_down(key),
                Event::KeyUp { keycode: Some(key), repeat: false, .. } => memory_bus.key_up(key),
                _ => {}
            }
        }

        // --- 2. Emulate One Frame ---
        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            // *** Handle Result from cpu.step ***
            let step_result = cpu.step(&mut memory_bus);

            match step_result {
                Ok(executed_cycles_u8) => {
                    let executed_cycles = executed_cycles_u8 as u32;
                    ppu.step(executed_cycles, &mut memory_bus);
                    apu.step(executed_cycles, &mut memory_bus);
                    // TODO: Step Timer
                    cycles_this_frame += executed_cycles;
                    // Check interrupts here or within cpu.step logic
                }
                Err(error_message) => {
                    // CPU encountered an error (e.g., unknown opcode)
                    eprintln!("\n==================== CPU Error ====================");
                    eprintln!("Emulator halted due to CPU error:");
                    eprintln!(" -> {}", error_message);
                    eprintln!("====================================================\n");

                    // Optionally, print CPU state here for debugging
                    // println!("CPU State at Error:\n{:?}", cpu);

                    break 'main_loop; // Stop the emulator
                }
            }
        }

        // --- 3. Update Debug Views ---
        ppu.render_vram_debug(&memory_bus);

        // --- 4. Drawing ---
        canvas.set_draw_color(Color::RGB(20, 20, 20));
        canvas.clear();

        draw_gb_screen(&mut canvas, ppu.get_frame_buffer(), 0, 0);

        let right_pane_x = (GB_SCREEN_WIDTH + PADDING) as i32;
        let vram_view_y = 0;
        draw_vram_debug(&mut canvas, ppu.get_vram_debug_buffer(), right_pane_x, vram_view_y);

        let input_view_y = vram_view_y + VRAM_VIEW_HEIGHT as i32 + PADDING as i32;
        draw_input_debug(&mut canvas, &memory_bus.joypad, right_pane_x, input_view_y);

        let input_debug_area_height: u32 = cmp::max(DPAD_AREA_HEIGHT, ACTION_AREA_HEIGHT); // Recalculate here for clarity
        let disasm_view_y = input_view_y + input_debug_area_height as i32 + PADDING as i32;
        if let Err(e) = draw_disassembly_debug(
            &mut canvas, &texture_creator, &font, &cpu, &memory_bus,
            right_pane_x, disasm_view_y, DISASM_LINES_BEFORE, DISASM_LINES_AFTER,
        ) {
            eprintln!("Error drawing disassembly: {}", e);
            // break 'main_loop; // Optional: Stop on drawing error
        }

        canvas.present();

        // --- 5. Frame Timing / Rate Limiting ---
        let elapsed_time = frame_start_time.elapsed();
        if elapsed_time < TARGET_FRAME_DURATION {
            let sleep_duration = TARGET_FRAME_DURATION.saturating_sub(elapsed_time);
            if sleep_duration > Duration::from_millis(1) {
                thread::sleep(sleep_duration.saturating_sub(Duration::from_millis(1)));
            }
            while Instant::now() < frame_start_time + TARGET_FRAME_DURATION {
                thread::yield_now();
            }
        }
        // Optional: else { eprintln!("Warning: Frame took too long: {:?}", elapsed_time); }

    } // End 'main_loop

    println!("Emulator stopped.");
    Ok(())
}