use super::constants::*;
use crate::memory_bus::MemoryBus;
use crate::memory_map; // Use memory_map for VRAM addresses

/// Renders the contents of VRAM tile data (0x8000-0x97FF) to the debug buffer.
pub(super) fn render_vram_debug(
    vram_debug_buffer: &mut [u8; VRAM_DEBUG_BUFFER_SIZE],
    memory_bus: &MemoryBus,
) {
    // Use a simple fixed palette mapping index 0-3 to shades 0-3 for clarity
    let get_debug_shade = |index: u8| index;

    for tile_idx in 0..NUM_TILES_TO_SHOW { // Render all 384 tiles
        // Calculate the base address for this tile (16 bytes per tile)
        // Tiles 0-255 are in $8000-$8FFF
        // Tiles 256-383 map to $8800-$97FF (which corresponds to tile IDs -128 to -1 / 128 to 255 in $8800 mode)
        // Let's just read sequentially from $8000 to $97FF for simplicity in the debug view.
        let tile_addr = memory_map::VRAM_START + (tile_idx * 16) as u16;

        // Calculate where this tile goes in the debug buffer grid
        let tile_grid_x = tile_idx % TILES_PER_ROW_DEBUG;
        let tile_grid_y = tile_idx / TILES_PER_ROW_DEBUG;
        let base_pixel_x = tile_grid_x * 8;
        let base_pixel_y = tile_grid_y * 8;

        // Render the 8x8 tile
        for y_in_tile in 0..8u16 {
            let row_addr = tile_addr + (y_in_tile * 2);

            // Check VRAM bounds before reading the row
            if row_addr < memory_map::VRAM_START || row_addr.wrapping_add(1) > memory_map::VRAM_END {
                // Tile address is out of VRAM range, fill with a default color?
                for y_fill in 0..8 {
                    for x_fill in 0..8 {
                        let px = base_pixel_x + x_fill;
                        let py = base_pixel_y + y_fill;
                        let buf_idx = py * VRAM_DEBUG_WIDTH + px;
                        if buf_idx < vram_debug_buffer.len() {
                            vram_debug_buffer[buf_idx] = 0; // White
                        }
                    }
                }
                break; // Stop processing this tile if address is invalid
            }

            let byte1 = memory_bus.read_byte(row_addr);
            let byte2 = memory_bus.read_byte(row_addr + 1);

            for x_in_tile in 0..8u8 {
                // Extract the color index for this pixel (Bit 7 left, Bit 0 right)
                let bit_pos = 7 - x_in_tile;
                let bit1 = (byte1 >> bit_pos) & 1;
                let bit2 = (byte2 >> bit_pos) & 1;
                let color_index = (bit2 << 1) | bit1;

                // Get the shade (0-3)
                let shade = get_debug_shade(color_index);

                // Calculate the pixel's position in the 1D debug buffer
                let pixel_x = base_pixel_x + (x_in_tile as usize);
                let pixel_y = base_pixel_y + (y_in_tile as usize);
                let buffer_index = pixel_y * VRAM_DEBUG_WIDTH + pixel_x;

                // Write to the debug buffer (if within bounds)
                if buffer_index < vram_debug_buffer.len() {
                    vram_debug_buffer[buffer_index] = shade;
                }
            } // end x_in_tile loop
        } // end y_in_tile loop
    } // end tile_idx loop
}