use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::ttf::Font;

// Import constants from the *binary's* constants module
use crate::constants; // <-- FIX 1

// Import types from the *library* crate
use boba::cpu::Cpu;
use boba::memory_bus::MemoryBus;
use boba::joypad::JoypadState; // <-- FIX 2
use boba::ppu; // Import ppu module for its constants

// --- Palettes --- (Keep or move to constants.rs)
const PALETTE: [Color; 4] = constants::PALETTE;
const DEBUG_PALETTE: [Color; 4] = constants::DEBUG_PALETTE;


/// Renders text to the canvas.
pub fn render_text(
    canvas: &mut Canvas<Window>,
    texture_creator: &TextureCreator<WindowContext>,
    font: &Font,
    text: &str,
    x: i32,
    y: i32,
    color: Color,
) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    let surface = font
        .render(text)
        .blended(color)
        .map_err(|e| e.to_string())?;
    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .map_err(|e| e.to_string())?;

    // Use query() to get the texture dimensions
    let texture_query = texture.query();
    let text_rect = Rect::new(x, y, texture_query.width, texture_query.height);

    canvas.copy(&texture, None, Some(text_rect))?;
    Ok(())
}


/// Draws the main Game Boy screen content.
pub fn draw_gb_screen(canvas: &mut Canvas<Window>, frame_buffer: &[u8], target_x: i32, target_y: i32) {
    let expected_len = (constants::GB_WIDTH * constants::GB_HEIGHT) as usize;
    if frame_buffer.len() != expected_len {
        eprintln!(
            "Error: Frame buffer size mismatch! Expected {}, got {}",
            expected_len,
            frame_buffer.len()
        );
        return;
    }
    for y in 0..constants::GB_HEIGHT {
        for x in 0..constants::GB_WIDTH {
            let index = (y * constants::GB_WIDTH + x) as usize;
            // Safely get pixel, default to 0, ensure index is valid
            let shade_index = frame_buffer.get(index).copied().unwrap_or(0) % 4;
            let color = PALETTE[shade_index as usize];

            canvas.set_draw_color(color);
            let rect = Rect::new(
                target_x + (x * constants::GB_SCALE_FACTOR) as i32,
                target_y + (y * constants::GB_SCALE_FACTOR) as i32,
                constants::GB_SCALE_FACTOR,
                constants::GB_SCALE_FACTOR,
            );
            // Use fill_rect which is usually faster for solid colors
            canvas.fill_rect(rect).unwrap_or_else(|e| eprintln!("Failed to draw GB pixel: {}", e));
        }
    }
}

/// Draws the VRAM tile data debug view.
pub fn draw_vram_debug(canvas: &mut Canvas<Window>, vram_buffer: &[u8], target_x: i32, target_y: i32) {
    // Use the constants exported from the ppu module if available and public
    let vram_debug_width = ppu::VRAM_DEBUG_WIDTH;
    let vram_debug_height = ppu::VRAM_DEBUG_HEIGHT;
    let expected_len = vram_debug_width * vram_debug_height;

    if vram_buffer.len() != expected_len {
        eprintln!(
            "Error: VRAM debug buffer size mismatch! Expected {}, got {}",
            expected_len,
            vram_buffer.len()
        );
        return;
    }

    for y in 0..vram_debug_height {
        for x in 0..vram_debug_width {
            let index = y * vram_debug_width + x;
            let shade_index = vram_buffer.get(index).copied().unwrap_or(0) % 4;
            let color = DEBUG_PALETTE[shade_index as usize];

            canvas.set_draw_color(color);
            let rect = Rect::new(
                target_x + (x as u32 * constants::VRAM_DEBUG_SCALE_FACTOR) as i32,
                target_y + (y as u32 * constants::VRAM_DEBUG_SCALE_FACTOR) as i32,
                constants::VRAM_DEBUG_SCALE_FACTOR,
                constants::VRAM_DEBUG_SCALE_FACTOR,
            );
             canvas.fill_rect(rect).unwrap_or_else(|e| eprintln!("Failed to draw VRAM pixel: {}", e));
        }
    }
}


/// Draws the joypad input state indicators.
pub fn draw_input_debug(
    canvas: &mut Canvas<Window>,
    joypad_state: &JoypadState,
    target_x: i32,
    target_y: i32,
) {
    let mut draw_indicator = |is_pressed: bool, x_offset: i32, y_offset: i32| {
        let color = if is_pressed {
            constants::DEBUG_INPUT_PRESSED_COLOR
        } else {
            constants::DEBUG_INPUT_RELEASED_COLOR
        };
        canvas.set_draw_color(color);
        let rect = Rect::new(
            target_x + x_offset,
            target_y + y_offset,
            constants::DEBUG_INPUT_BOX_SIZE,
            constants::DEBUG_INPUT_BOX_SIZE,
        );
        canvas.fill_rect(rect).unwrap_or_else(|e| eprintln!("Failed to draw input indicator: {}", e));
    };

    let pad_step = (constants::DEBUG_INPUT_BOX_SIZE + constants::DEBUG_INPUT_PADDING) as i32;
    let dpad_center_x = pad_step;
    let dpad_center_y = pad_step;

    // Draw D-Pad
    draw_indicator(joypad_state.up, dpad_center_x, dpad_center_y - pad_step);
    draw_indicator(joypad_state.down, dpad_center_x, dpad_center_y + pad_step);
    draw_indicator(joypad_state.left, dpad_center_x - pad_step, dpad_center_y);
    draw_indicator(joypad_state.right, dpad_center_x + pad_step, dpad_center_y);

    // Draw Action buttons
    let action_start_x = constants::DPAD_AREA_WIDTH as i32 + constants::PADDING as i32;
    let action_y1 = 0;
    let action_y2 = pad_step;
    draw_indicator(joypad_state.b, action_start_x, action_y1);
    draw_indicator(joypad_state.a, action_start_x + pad_step, action_y1);
    draw_indicator(joypad_state.select, action_start_x, action_y2);
    draw_indicator(joypad_state.start, action_start_x + pad_step, action_y2);
}


/// Draws the CPU disassembly debug view.
pub fn draw_disassembly_debug(
    canvas: &mut Canvas<Window>,
    texture_creator: &TextureCreator<WindowContext>,
    font: &Font,
    cpu: &Cpu,
    bus: &MemoryBus,
    target_x: i32,
    target_y: i32,
) -> Result<(), String> {
    // Use constants directly
    let lines_before = constants::DISASM_LINES_BEFORE;
    let lines_after = constants::DISASM_LINES_AFTER;
    let area_width = constants::DISASM_AREA_WIDTH;
    let area_height = constants::DISASM_AREA_HEIGHT;
    let line_height = constants::DISASM_LINE_HEIGHT;

    // Draw background
    canvas.set_draw_color(constants::DEBUG_BACKGROUND_COLOR);
    let bg_rect = Rect::new(target_x, target_y, area_width, area_height);
    canvas.fill_rect(bg_rect).map_err(|e| e.to_string())?;

    let current_pc = cpu.pc(); // Use accessor
    let total_lines = lines_before + 1 + lines_after;
    let mut instructions: Vec<(u16, String)> = Vec::with_capacity(total_lines);

    // Disassemble Forwards
    let mut current_addr = current_pc;
    for _ in 0..=lines_after {
        let (disasm_text, instr_len) = cpu.disassemble_instruction(current_addr, bus);
        instructions.push((current_addr, disasm_text));
        if instr_len == 0 { // Avoid infinite loop on zero-length instruction (shouldn't happen)
            current_addr = current_addr.wrapping_add(1); // Move forward anyway
        } else {
            current_addr = current_addr.wrapping_add(instr_len as u16);
        }
        if instructions.len() > total_lines * 2 { break; } // Safety break
    }

    // Disassemble Backwards (Approximate)
    let mut start_addr = current_pc;
    let mut backward_instructions = Vec::new();
    for _ in 0..lines_before {
        let mut found_prev = false;
        // Try guessing previous instruction lengths (1 to 3 bytes)
        for offset_guess in (1..=3).rev() {
            if let Some(prev_addr_guess) = start_addr.checked_sub(offset_guess) {
                 // Ensure guess is valid address range if needed, though wrap around is fine here
                let (_, len_guess) = cpu.disassemble_instruction(prev_addr_guess, bus);
                if len_guess == offset_guess as u8 && len_guess != 0 {
                    start_addr = prev_addr_guess;
                    let (disasm_text, _) = cpu.disassemble_instruction(start_addr, bus);
                    backward_instructions.push((start_addr, disasm_text));
                    found_prev = true;
                    break; // Found a likely previous instruction
                }
            }
        }
        if !found_prev { break; } // Couldn't find a valid previous instruction
    }
    // Combine backward and forward disassembly, ensuring PC is roughly centered
    backward_instructions.reverse();
    instructions = [backward_instructions, instructions].concat();


    // Render the collected lines, trying to keep PC centered
    let mut current_y = target_y;
    let pc_index_maybe = instructions.iter().position(|(addr, _)| *addr == current_pc);

    let start_render_idx = if let Some(pc_index) = pc_index_maybe {
        pc_index.saturating_sub(lines_before)
    } else {
         // If PC wasn't found (e.g., during backward scan fail), start from beginning
        0
    };

    for (idx, (addr, text)) in instructions.iter().enumerate().skip(start_render_idx) {
        // Don't render more lines than fit in the area
        if (idx - start_render_idx) >= total_lines { break; }

        let display_text = format!("{:04X}: {}", addr, text);
        let color = if *addr == current_pc {
            constants::DEBUG_PC_COLOR
        } else {
            constants::DEBUG_TEXT_COLOR
        };

        render_text(
            canvas,
            texture_creator,
            font,
            &display_text,
            target_x + 5, // Small padding
            current_y,
            color,
        )?;
        current_y += line_height as i32;
    }

    Ok(())
}