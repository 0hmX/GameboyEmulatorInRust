use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::Window;

use super::constants; // Use constants from the same app module
use boba::cpu::Cpu;
use boba::joypad::JoypadState;
use boba::memory_bus::MemoryBus; // Assuming this holds button states

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
            if index >= frame_buffer.len() {
                continue;
            } // Prevent out-of-bounds if buffer is too small

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
    canvas.fill_rect(Rect::new(
        x,
        y,
        constants::VRAM_VIEW_WIDTH,
        constants::VRAM_VIEW_HEIGHT,
    ))?;

    for py in 0..native_height {
        for px in 0..native_width {
            let index = (py * native_width + px) as usize;
            if index >= vram_buffer.len() {
                continue;
            } // Prevent out-of-bounds

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
    canvas.fill_rect(Rect::new(
        pane_x,
        pane_y,
        constants::DISASM_AREA_WIDTH,
        constants::DISASM_AREA_HEIGHT,
    ))?;

    let pc = cpu.pc();
    let mut current_addr = pc; // We'll calculate the start address later

    // --- Attempt to find a reasonable start address ---
    // This is tricky without full backward disassembly. We'll iterate backwards
    // a few times, disassembling each instruction to find a likely start point.
    // This is better than guessing fixed offsets but still not perfect.
    let mut start_addr = pc;
    for _ in 0..constants::DISASM_LINES_BEFORE {
        // To step back accurately, we'd ideally need to know the length of the *previous*
        // instruction. This requires a more complex backward scan or heuristics.
        // A simpler, less accurate approach is to guess an average length (e.g., 2)
        // or try disassembling from addr-1, addr-2, addr-3 and see if they *end* at `start_addr`.
        //
        // Let's stick to a simpler (but still imperfect) fixed offset for now,
        // as true backward disassembly is complex. The forward rendering below is correct.
        start_addr = start_addr.saturating_sub(2); // Guess average 2 bytes/instruction backward
                                                   // TODO: Implement more robust backward seeking if needed.
    }
    // Ensure start_addr doesn't go below 0 after subtractions
    // The simple subtraction might land us mid-instruction. We accept this limitation for now.

    current_addr = start_addr;

    // --- Disassemble and Draw Lines Forward ---
    for i in 0..constants::DISASM_TOTAL_LINES {
        let line_addr = current_addr; // Address for this line

        // Get the disassembled instruction and its actual length
        let (mnemonic, instr_len_u8) = cpu.disassemble_instruction(line_addr, memory_bus);
        let instr_len = instr_len_u8 as u16;

        // Read the raw bytes for this instruction to display them
        // let mut bytes_str = String::new();
        // let mut raw_bytes: Vec<u8> = Vec::with_capacity(instr_len as usize);
        // for offset in 0..instr_len {
        //     // Ensure we don't wrap around address space excessively if near 0xFFFF
        //     let byte_addr = line_addr.wrapping_add(offset);
        //     let byte = memory_bus.read_byte(byte_addr);
        //     raw_bytes.push(byte);
        //     bytes_str.push_str(&format!("{:02X} ", byte));
        // }

        // Pad the byte string so the mnemonics align nicely
        // Max standard instruction length is 3 bytes (XX XX XX ). CB prefix instructions are 2 bytes (CB XX ).
        // We need space for up to 3 hex pairs + 3 spaces = 9 characters.
        // let byte_display_width = constants::MAX_INSTR_BYTES * 3; // e.g., 3 bytes * (2 chars + 1 space) = 9
        // while bytes_str.len() < byte_display_width {
        //     bytes_str.push(' ');
        // }
        // Ensure it doesn't exceed the width if somehow instr_len was > MAX_INSTR_BYTES
        // bytes_str.truncate(byte_display_width);


        // Format the complete line: Address: Bytes Mnemonic
        let disasm_text = format!("0x{:04X}: {}", line_addr, mnemonic);

        // Determine text color (highlight the line at PC)
        let text_color = if line_addr == pc {
            constants::DEBUG_PC_COLOR
        } else {
            constants::DEBUG_TEXT_COLOR
        };

        // Render the text line
        let surface = font
            .render(&disasm_text)
            .blended(text_color)
            .map_err(|e| e.to_string())?;

        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .map_err(|e| e.to_string())?;

        let TextureQuery {
            width: text_width,
            height: text_height,
            ..
        } = texture.query();

        // Calculate Y position for this line
        let current_line_y = pane_y + (i as i32 * constants::DISASM_LINE_HEIGHT as i32);

        // Define destination rectangle for the text
        let dest_rect = Rect::new(
            pane_x + 5, // Add some left padding
            current_line_y,
            text_width,
            text_height,
        );

        // Copy the texture to the canvas
        canvas.copy(&texture, None, Some(dest_rect))?;

        // Advance address for the next line using the *actual* instruction length
        current_addr = current_addr.wrapping_add(instr_len);

        // Safeguard against zero-length instructions (shouldn't happen with valid disassembler)
        if instr_len == 0 {
            // If this happens, the disassembler has a bug or encountered truly invalid data.
            // We advance by 1 to avoid an infinite loop.
            // Log this error if possible.
            // eprintln!("Warning: Disassembled instruction at ${:04X} reported zero length.", line_addr);
            current_addr = current_addr.wrapping_add(1);
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
    canvas.fill_rect(Rect::new(
        x,
        y,
        constants::INPUT_DEBUG_AREA_WIDTH,
        constants::INPUT_DEBUG_AREA_HEIGHT,
    ))?;

    let box_size = constants::DEBUG_INPUT_BOX_SIZE;
    let pad = constants::DEBUG_INPUT_PADDING;

    // --- D-Pad ---
    let dpad_base_x = x + 5; // Add some padding from pane edge
    let dpad_base_y = y + (constants::INPUT_DEBUG_AREA_HEIGHT as i32 / 2)
        - (constants::DPAD_AREA_HEIGHT as i32 / 2); // Center vertically

    // Up
    let up_color = if joypad_state.up {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(up_color);
    canvas.fill_rect(Rect::new(
        dpad_base_x + box_size as i32 + pad as i32,
        dpad_base_y,
        box_size,
        box_size,
    ))?;

    // Down
    let down_color = if joypad_state.down {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(down_color);
    canvas.fill_rect(Rect::new(
        dpad_base_x + box_size as i32 + pad as i32,
        dpad_base_y + 2 * (box_size as i32 + pad as i32),
        box_size,
        box_size,
    ))?;

    // Left
    let left_color = if joypad_state.left {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(left_color);
    canvas.fill_rect(Rect::new(
        dpad_base_x,
        dpad_base_y + box_size as i32 + pad as i32,
        box_size,
        box_size,
    ))?;

    // Right
    let right_color = if joypad_state.right {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(right_color);
    canvas.fill_rect(Rect::new(
        dpad_base_x + 2 * (box_size as i32 + pad as i32),
        dpad_base_y + box_size as i32 + pad as i32,
        box_size,
        box_size,
    ))?;

    // --- Action Buttons ---
    let buttons_base_x =
        dpad_base_x + constants::DPAD_AREA_WIDTH as i32 + constants::PADDING as i32;
    let buttons_base_y = y + (constants::INPUT_DEBUG_AREA_HEIGHT as i32 / 2)
        - (constants::BUTTONS_AREA_HEIGHT as i32 / 2); // Center vertically

    // B
    let b_color = if joypad_state.b {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(b_color);
    canvas.fill_rect(Rect::new(
        buttons_base_x,
        buttons_base_y + box_size as i32 / 2,
        box_size,
        box_size,
    ))?; // Align B slightly lower maybe

    // A
    let a_color = if joypad_state.a {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(a_color);
    canvas.fill_rect(Rect::new(
        buttons_base_x + box_size as i32 + pad as i32,
        buttons_base_y,
        box_size,
        box_size,
    ))?; // Align A slightly higher

    // Select
    let select_color = if joypad_state.select {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(select_color);
    canvas.fill_rect(Rect::new(
        buttons_base_x + box_size as i32 / 2,
        buttons_base_y + box_size as i32 + pad as i32 + box_size as i32 / 2,
        box_size + pad,
        box_size / 2,
    ))?; // Small rect for select

    // Start
    let start_color = if joypad_state.start {
        constants::DEBUG_INPUT_PRESSED_COLOR
    } else {
        constants::DEBUG_INPUT_RELEASED_COLOR
    };
    canvas.set_draw_color(start_color);
    canvas.fill_rect(Rect::new(
        buttons_base_x + box_size as i32 + pad as i32 * 2,
        buttons_base_y + box_size as i32 + pad as i32 + box_size as i32 / 2,
        box_size + pad,
        box_size / 2,
    ))?; // Small rect for start

    Ok(())
}
