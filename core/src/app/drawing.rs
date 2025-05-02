use sdl2::render::{Canvas, TextureCreator, TextureQuery};
use sdl2::video::Window;
use sdl2::rect::{Rect, Point};
use sdl2::pixels::Color;
use sdl2::ttf::Font;

use super::constants; // Use constants from the same app module
use boba::cpu::Cpu;
use boba::memory_bus::MemoryBus;
use boba::joypad::JoypadState; // Assuming this holds button states

/// Draws the scaled Game Boy screen content to the canvas.
pub fn draw_gb_screen(
    canvas: &mut Canvas<Window>,
    frame_buffer: &[u8], // Expects buffer of palette indices (0-3)
    x: i32,
    y: i32,
) -> Result<(), String> {
    // Ensure the buffer has the expected size
    if frame_buffer.len() != (constants::GB_WIDTH * constants::GB_HEIGHT) as usize {
        // This check might be too strict if the buffer format changes, but good for safety
        // eprintln!( // Use eprintln for errors, not println
        //     "Warning: Frame buffer size mismatch. Expected {}, got {}.",
        //     constants::GB_WIDTH * constants::GB_HEIGHT,
        //     frame_buffer.len()
        // );
        // Allow drawing anyway, but it might look wrong or panic if smaller
    }

    let scale = constants::GB_SCALE_FACTOR as i32;
    let scaled_width = constants::GB_WIDTH as i32 * scale;
    let scaled_height = constants::GB_HEIGHT as i32 * scale;

    // Optional: Draw a background or border for the GB screen area
    canvas.set_draw_color(Color::RGB(10, 10, 10));
    canvas.fill_rect(Rect::new(x, y, scaled_width as u32, scaled_height as u32))?;

    for py in 0..constants::GB_HEIGHT {
        for px in 0..constants::GB_WIDTH {
            let index = (py * constants::GB_WIDTH + px) as usize;
            if index >= frame_buffer.len() { continue; } // Prevent out-of-bounds if buffer is too small

            let color_index = frame_buffer[index];
            let color = constants::PALETTE[color_index as usize % 4]; // Modulo 4 for safety

            canvas.set_draw_color(color);

            let dest_x = x + (px as i32 * scale);
            let dest_y = y + (py as i32 * scale);
            let dest_rect = Rect::new(dest_x, dest_y, scale as u32, scale as u32);

            canvas.fill_rect(dest_rect)?;
        }
    }
    Ok(())
}

/// Draws the VRAM tile data debug view.
pub fn draw_vram_debug(
    canvas: &mut Canvas<Window>,
    vram_buffer: &[u8], // Expects buffer of palette indices (0-3) for the debug view pixels
    x: i32,
    y: i32,
) -> Result<(), String> {
    // Use the native dimensions from the PPU constants (if available) or recalculate
    let native_width = constants::VRAM_VIEW_WIDTH / constants::VRAM_DEBUG_SCALE_FACTOR;
    let native_height = constants::VRAM_VIEW_HEIGHT / constants::VRAM_DEBUG_SCALE_FACTOR;

    if vram_buffer.len() != (native_width * native_height) as usize {
         // eprintln!(
         //     "Warning: VRAM debug buffer size mismatch. Expected {}, got {}.",
         //      native_width * native_height,
         //      vram_buffer.len()
         // );
         // Allow drawing anyway
    }

    let scale = constants::VRAM_DEBUG_SCALE_FACTOR as i32;

    // Draw background for VRAM view area
    canvas.set_draw_color(constants::DEBUG_BACKGROUND_COLOR);
    canvas.fill_rect(Rect::new(x, y, constants::VRAM_VIEW_WIDTH, constants::VRAM_VIEW_HEIGHT))?;


    for py in 0..native_height {
        for px in 0..native_width {
            let index = (py * native_width + px) as usize;
             if index >= vram_buffer.len() { continue; } // Prevent out-of-bounds

            let color_index = vram_buffer[index];
            let color = constants::DEBUG_PALETTE[color_index as usize % 4]; // Use debug palette

            canvas.set_draw_color(color);

            let dest_x = x + (px as i32 * scale);
            let dest_y = y + (py as i32 * scale);
            let dest_rect = Rect::new(dest_x, dest_y, scale as u32, scale as u32);

            canvas.fill_rect(dest_rect)?;
        }
    }

    Ok(())
}


/// Draws the disassembly debug view around the current PC.
pub fn draw_disassembly_debug(
    canvas: &mut Canvas<Window>,
    texture_creator: &TextureCreator<sdl2::video::WindowContext>,
    font: &Font,
    cpu: &Cpu,
    memory_bus: &MemoryBus,
    pane_x: i32,
    pane_y: i32,
) -> Result<(), String> {
    // Draw background for the disassembly pane
    canvas.set_draw_color(constants::DEBUG_BACKGROUND_COLOR);
    canvas.fill_rect(Rect::new(pane_x, pane_y, constants::DISASM_AREA_WIDTH, constants::DISASM_AREA_HEIGHT))?;

    let pc = cpu.pc();
    let mut current_addr = pc; // Start disassembling near PC

    // --- VERY Simple Disassembly Logic (Placeholder) ---
    // A real disassembler is complex. This just shows bytes and a NOP example.
    // It doesn't handle variable instruction lengths correctly for seeking backwards.
    // We'll just display lines starting near PC.
    // Try to start a few instructions before PC (rough estimate)
    // This backward seeking is inherently flawed without full disassembly analysis.
    let mut start_addr = pc.saturating_sub((constants::DISASM_LINES_BEFORE * 2) as u16); // Guess 2 bytes/instr avg

    for i in 0..constants::DISASM_TOTAL_LINES {
        let line_addr = start_addr;

        // Fetch bytes (handle potential read errors gracefully, maybe show "??")
        let byte1 = memory_bus.read_byte(start_addr);
        let byte2 = memory_bus.read_byte(start_addr.wrapping_add(1));
        let byte3 = memory_bus.read_byte(start_addr.wrapping_add(2));

        // Placeholder disassembly: just show address and bytes
        let mut disasm_text = format!("${:04X}: {:02X} {:02X} {:02X}", line_addr, byte1, byte2, byte3);
        let mut instr_len: u16 = 1; // Default length

        // Extremely basic instruction check (replace with actual disassembler call)
        if byte1 == 0x00 { // NOP
            disasm_text.push_str(" NOP");
            instr_len = 1;
        } else if byte1 == 0xC3 { // JP nn
             let target = u16::from_le_bytes([byte2, byte3]);
             disasm_text = format!("${:04X}: {:02X} {:02X} {:02X} JP ${:04X}", line_addr, byte1, byte2, byte3, target);
             instr_len = 3;
        } else {
             disasm_text.push_str(" ..."); // Indicate unknown instruction
             instr_len = 1; // Guess length
        }


        // Determine text color
        let text_color = if line_addr == pc {
            constants::DEBUG_PC_COLOR
        } else {
            constants::DEBUG_TEXT_COLOR
        };

        // Render the text line
        let surface = font.render(&disasm_text)
            .blended(text_color) // Use blended for potentially better quality
            .map_err(|e| e.to_string())?;

        let texture = texture_creator.create_texture_from_surface(&surface)
            .map_err(|e| e.to_string())?;

        let TextureQuery { width: text_width, height: text_height, .. } = texture.query();

        // Calculate Y position for this line **USING i32 CASTING**
        let current_line_y = pane_y + (i as i32 * constants::DISASM_LINE_HEIGHT as i32);

        // Define destination rectangle for the text
        let dest_rect = Rect::new(pane_x + 5, current_line_y, text_width, text_height); // Add padding

        // Copy the texture to the canvas
        canvas.copy(&texture, None, Some(dest_rect))?;

        // Advance address for the next line based on *guessed* instruction length
         start_addr = start_addr.wrapping_add(instr_len);

        // Simple protection against infinite loops if instr_len is 0 (shouldn't happen)
        if instr_len == 0 {
             start_addr = start_addr.wrapping_add(1);
        }
    }

    Ok(())
}

/// Draws the input state debug view.
pub fn draw_input_debug(
    canvas: &mut Canvas<Window>,
    joypad_state: &JoypadState, // Expecting the state struct
    x: i32,
    y: i32,
) -> Result<(), String> {
    // Draw background for input view area
    canvas.set_draw_color(constants::DEBUG_BACKGROUND_COLOR);
    canvas.fill_rect(Rect::new(x, y, constants::INPUT_DEBUG_AREA_WIDTH, constants::INPUT_DEBUG_AREA_HEIGHT))?;


    let box_size = constants::DEBUG_INPUT_BOX_SIZE;
    let pad = constants::DEBUG_INPUT_PADDING;

    // --- D-Pad ---
    let dpad_base_x = x + 5; // Add some padding from pane edge
    let dpad_base_y = y + (constants::INPUT_DEBUG_AREA_HEIGHT as i32 / 2) - (constants::DPAD_AREA_HEIGHT as i32 / 2); // Center vertically

    // Up
    let up_color = if joypad_state.up { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(up_color);
    canvas.fill_rect(Rect::new(dpad_base_x + box_size as i32 + pad as i32, dpad_base_y, box_size, box_size))?;

    // Down
    let down_color = if joypad_state.down { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(down_color);
    canvas.fill_rect(Rect::new(dpad_base_x + box_size as i32 + pad as i32, dpad_base_y + 2 * (box_size as i32 + pad as i32), box_size, box_size))?;

    // Left
    let left_color = if joypad_state.left { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(left_color);
    canvas.fill_rect(Rect::new(dpad_base_x, dpad_base_y + box_size as i32 + pad as i32, box_size, box_size))?;

    // Right
    let right_color = if joypad_state.right { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(right_color);
    canvas.fill_rect(Rect::new(dpad_base_x + 2 * (box_size as i32 + pad as i32), dpad_base_y + box_size as i32 + pad as i32, box_size, box_size))?;


    // --- Action Buttons ---
    let buttons_base_x = dpad_base_x + constants::DPAD_AREA_WIDTH as i32 + constants::PADDING as i32;
    let buttons_base_y = y + (constants::INPUT_DEBUG_AREA_HEIGHT as i32 / 2) - (constants::BUTTONS_AREA_HEIGHT as i32 / 2); // Center vertically

    // B
    let b_color = if joypad_state.b { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(b_color);
    canvas.fill_rect(Rect::new(buttons_base_x, buttons_base_y + box_size as i32 / 2, box_size, box_size))?; // Align B slightly lower maybe

    // A
    let a_color = if joypad_state.a { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(a_color);
    canvas.fill_rect(Rect::new(buttons_base_x + box_size as i32 + pad as i32, buttons_base_y, box_size, box_size))?; // Align A slightly higher

    // Select
    let select_color = if joypad_state.select { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(select_color);
    canvas.fill_rect(Rect::new(buttons_base_x + box_size as i32 / 2, buttons_base_y + box_size as i32 + pad as i32 + box_size as i32 / 2, box_size + pad, box_size / 2))?; // Small rect for select

     // Start
    let start_color = if joypad_state.start { constants::DEBUG_INPUT_PRESSED_COLOR } else { constants::DEBUG_INPUT_RELEASED_COLOR };
    canvas.set_draw_color(start_color);
    canvas.fill_rect(Rect::new(buttons_base_x + box_size as i32 + pad as i32 * 2, buttons_base_y + box_size as i32 + pad as i32 + box_size as i32 / 2, box_size + pad, box_size / 2))?; // Small rect for start

    Ok(())
}