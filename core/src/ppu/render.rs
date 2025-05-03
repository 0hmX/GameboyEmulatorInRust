use super::constants::*;
use super::state::PpuState;
use crate::memory_bus::MemoryBus;
use crate::memory_map;

pub(super) fn render_scanline(
    line_buffer: &mut [u8; GB_WIDTH],
    state: &PpuState,
    memory_bus: &MemoryBus,
) {
    let y = state.current_scanline;
    if y >= GB_HEIGHT as u8 {
        return;
    }

    // Read necessary registers for rendering this line
    // Note: state.lcdc and state.stat are already cached
    let lcdc = state.lcdc;
    let scy = memory_bus.read_byte(memory_map::SCY_ADDR);
    let scx = memory_bus.read_byte(memory_map::SCX_ADDR);
    let wy = memory_bus.read_byte(memory_map::WY_ADDR);
    let wx = memory_bus.read_byte(memory_map::WX_ADDR);
    let bgp = memory_bus.read_byte(memory_map::BGP_ADDR);

    let window_enabled = (lcdc & (1 << LCDC_WINDOW_ENABLE)) != 0;
    let window_visible_y = window_enabled && y >= wy;

    // Determine if the background/window layer itself is enabled (LCDC Bit 0)
    let bg_win_display_enabled = (lcdc & (1 << LCDC_BG_WIN_ENABLE_PRIORITY)) != 0;

    // Fetch sprites visible on this scanline *once* before iterating through pixels
    // (More efficient than fetching per pixel)
    let sprites = fetch_scanline_sprites(state, memory_bus);

    for x in 0..GB_WIDTH as u8 {
        let x_usize = x as usize;

        let final_pixel_color_idx;
        let bg_win_pixel_idx;

        // --- Render Background / Window ---
        if bg_win_display_enabled {
            let window_covers_pixel = window_visible_y && x >= wx.saturating_sub(7);

            let tile_map_pixel_idx = if window_covers_pixel {
                fetch_window_pixel_index(x, y, wx, wy, lcdc, memory_bus)
            } else {
                fetch_bg_pixel_index(x, y, scx, scy, lcdc, memory_bus)
            };

            bg_win_pixel_idx = tile_map_pixel_idx; // Save the index
            final_pixel_color_idx = tile_map_pixel_idx;
        } else {
            // If BG/Win display is off (LCDC Bit 0 = 0), the whole layer is color 0
            bg_win_pixel_idx = 0;
            final_pixel_color_idx = 0;
        }

        let final_pixel_color = get_color_from_palette(final_pixel_color_idx, bgp);

        // --- Render Sprites (if enabled) ---
        let mut sprite_color_override = final_pixel_color; // Start with BG/Win color
        let mut sprite_found = false;

        if (lcdc & (1 << LCDC_OBJ_ENABLE)) != 0 {
            // Find the highest priority *visible* sprite at this X coordinate
            let mut winning_sprite_pixel_idx = 0;
            let mut winning_sprite_palette = 0;
            let mut winning_sprite_oam_prio = false;
            let mut best_sprite_x = 255; // Use 255 for comparison, lower X wins

            for sprite in &sprites {
                // Is this sprite horizontally covering the current pixel 'x'?
                let effective_x = sprite.x_pos.wrapping_sub(8);
                 if x >= effective_x && x < effective_x.wrapping_add(8) {
                    // Check horizontal position only once per sprite
                    if sprite.x_pos < best_sprite_x { // Lower X wins priority
                        // Calculate pixel within this potentially winning sprite
                        let col_in_tile = if sprite.x_flip {
                            7 - (x - effective_x)
                        } else {
                            x - effective_x
                        };

                         let sprite_pixel_idx = get_sprite_tile_pixel_index(sprite, col_in_tile, memory_bus);

                        if sprite_pixel_idx != 0 { // Only consider non-transparent pixels
                            // This sprite is visible and potentially the winner
                            winning_sprite_pixel_idx = sprite_pixel_idx;
                            winning_sprite_palette = sprite.palette_reg_value;
                            winning_sprite_oam_prio = sprite.bg_priority;
                            best_sprite_x = sprite.x_pos; // Update best X found
                            sprite_found = true;
                        }
                    }
                 }
            } // End of sprite loop for this pixel

            // If a visible sprite was found for this pixel, apply priority logic
            if sprite_found {
                let sprite_color = get_color_from_palette(winning_sprite_pixel_idx, winning_sprite_palette);
                let bg_win_is_transparent = bg_win_pixel_idx == 0;

                // Condition: Sprite is drawn if...
                // - BG/Win master display is disabled OR
                // - Sprite has OAM priority over BG (OAM flag=0) OR
                // - BG/Win pixel IS color 0 (sprite always draws over BG color 0)
                let sprite_wins_priority =
                    !bg_win_display_enabled || !winning_sprite_oam_prio || bg_win_is_transparent;

                if sprite_wins_priority {
                    sprite_color_override = sprite_color;
                }
            }
        } // End of sprite handling enable check

        line_buffer[x_usize] = sprite_color_override;
    }
}

/// Helper to get color shade from pixel index (0-3) and palette register value.
#[inline(always)]
pub(super) fn get_color_from_palette(pixel_index: u8, palette_reg: u8) -> u8 {
    // Extracts the 2-bit color specified by index from the 8-bit palette register
    (palette_reg >> (pixel_index * 2)) & 0b11
}


/// Fetches the raw pixel index (0-3) for the background at screen coordinates (x, y).
#[inline]
fn fetch_bg_pixel_index(
    screen_x: u8,
    screen_y: u8,
    scx: u8,
    scy: u8,
    lcdc: u8,
    memory_bus: &MemoryBus,
) -> u8 {
    // Calculate pixel coordinates within the full 256x256 background map
    let map_x = screen_x.wrapping_add(scx);
    let map_y = screen_y.wrapping_add(scy);

    // Determine which 32x32 tile map to use
    let map_base_addr = if (lcdc & (1 << LCDC_BG_MAP_AREA)) == 0 {
        0x9800
    } else {
        0x9C00
    };

    // Calculate the tile index within the map
    let tile_x = (map_x / 8) as u16;
    let tile_y = (map_y / 8) as u16;
    let tile_map_offset = tile_y * 32 + tile_x;
    let tile_id_addr = map_base_addr + tile_map_offset;

    // Read the tile ID (index) from the map
    let tile_id = memory_bus.read_byte(tile_id_addr);

    // Calculate the address of the tile's pattern data in VRAM
    let tile_addr = calculate_tile_data_addr(tile_id, lcdc, memory_bus);

    // Calculate the specific row within the 8x8 tile
    let row_in_tile = (map_y % 8) as u16;
    let row_addr = tile_addr + row_in_tile * 2;

    // Get the pixel data for the specific column within the tile row
    let col_in_tile = 7 - (map_x % 8); // Bit 7 is left, 0 is right
    get_tile_row_pixel_index(row_addr, col_in_tile, memory_bus)
}

/// Fetches the raw pixel index (0-3) for the window layer at screen coordinates (x, y).
/// Assumes window visibility checks (Y and X ranges) are done beforehand.
#[inline]
fn fetch_window_pixel_index(
    screen_x: u8,
    screen_y: u8,
    wx: u8,
    wy: u8,
    lcdc: u8,
    memory_bus: &MemoryBus,
) -> u8 {
    // Calculate pixel coordinates relative to the window's top-left corner
    let win_x = screen_x - wx.saturating_sub(7);
    let win_y = screen_y - wy; // Assumes screen_y >= wy

    // Determine which 32x32 tile map to use
    let map_base_addr = if (lcdc & (1 << LCDC_WINDOW_MAP_AREA)) == 0 {
        0x9800
    } else {
        0x9C00
    };

    // Calculate the tile index within the map
    let tile_x = (win_x / 8) as u16;
    let tile_y = (win_y / 8) as u16;
    let tile_map_offset = tile_y * 32 + tile_x;
    let tile_id_addr = map_base_addr + tile_map_offset;

    // Read the tile ID (index) from the map
    let tile_id = memory_bus.read_byte(tile_id_addr);

    // Calculate the address of the tile's pattern data in VRAM
    let tile_addr = calculate_tile_data_addr(tile_id, lcdc, memory_bus);

    // Calculate the specific row within the 8x8 tile
    let row_in_tile = (win_y % 8) as u16;
    let row_addr = tile_addr + row_in_tile * 2;

    // Get the pixel data for the specific column within the tile row
    let col_in_tile = 7 - (win_x % 8); // Bit 7 is left, 0 is right
    get_tile_row_pixel_index(row_addr, col_in_tile, memory_bus)
}

/// Calculates the starting address of a tile's pattern data based on its ID and LCDC Tile Data Area setting.
#[inline]
fn calculate_tile_data_addr(tile_id: u8, lcdc: u8, _memory_bus: &MemoryBus) -> u16 {
    if (lcdc & (1 << LCDC_TILE_DATA_AREA)) == 0 {
        // Addressing mode $8800: ID is treated as signed offset from $9000
        // $9000 + (tile_id as i8 * 16)
        let base_addr = 0x9000u16;
        let offset = (tile_id as i8 as i16) * 16;
        base_addr.wrapping_add(offset as u16)
    } else {
        // Addressing mode $8000: ID is treated as unsigned offset from $8000
        // $8000 + (tile_id as u16 * 16)
        0x8000u16 + (tile_id as u16 * 16)
    }
}

/// Reads the two bytes for a tile row and extracts the pixel index (0-3) for a given column.
#[inline]
fn get_tile_row_pixel_index(row_addr: u16, col_in_tile: u8, memory_bus: &MemoryBus) -> u8 {
     // Check VRAM bounds before reading
    if row_addr < memory_map::VRAM_START || row_addr.wrapping_add(1) > memory_map::VRAM_END {
        return 0; // Return transparent if address is invalid
    }

    let byte1 = memory_bus.read_byte(row_addr);
    let byte2 = memory_bus.read_byte(row_addr + 1);

    // Extract the two bits for the pixel's color index
    let bit1 = (byte1 >> col_in_tile) & 1;
    let bit2 = (byte2 >> col_in_tile) & 1;
    (bit2 << 1) | bit1 // Combine bits: bit2 is MSB, bit1 is LSB
}


// --- Sprite Fetching ---

/// Represents the relevant data for a sprite potentially visible on the current scanline.
#[derive(Debug)]
struct SpriteInfo {
    oam_index: u8,
    y_pos: u8, // OAM Y value (screen Y + 16)
    x_pos: u8, // OAM X value (screen X + 8)
    tile_index: u8, // Base tile index
    attributes: u8,
    // Pre-calculated attributes for rendering:
    height: u8,
    palette_reg_value: u8,
    x_flip: bool,
    y_flip: bool,
    bg_priority: bool, // True if BG colors 1-3 have priority over this sprite
}

/// Fetches up to 10 sprites that are visible on the current scanline.
/// Sprites are sorted by X-coordinate (ascending), then OAM index (ascending).
fn fetch_scanline_sprites(state: &PpuState, memory_bus: &MemoryBus) -> Vec<SpriteInfo> {
    let mut visible_sprites = Vec::with_capacity(10);
    let current_y = state.current_scanline;
    let sprite_height = if (state.lcdc & (1 << LCDC_OBJ_SIZE)) != 0 { 16 } else { 8 };

    // Read OBP0 and OBP1 once
    let obp0 = memory_bus.read_byte(memory_map::OBP0_ADDR);
    let obp1 = memory_bus.read_byte(memory_map::OBP1_ADDR);

    for i in 0..40 { // Iterate through all 40 OAM entries
        let oam_addr = memory_map::OAM_START + (i * 4);
        let sprite_y = memory_bus.read_byte(oam_addr);     // Y pos + 16
        let sprite_x = memory_bus.read_byte(oam_addr + 1); // X pos + 8

        // Check basic visibility conditions (on-screen position)
        if sprite_x == 0 || sprite_x >= (GB_WIDTH as u8 + 8) { continue; } // Off-screen horizontally
        if sprite_y == 0 || sprite_y >= (GB_HEIGHT as u8 + 16) { continue; } // Off-screen vertically (using OAM value)

        // Check vertical intersection with current scanline
        let effective_y = sprite_y.wrapping_sub(16); // Screen Y coordinate of top edge
        if current_y >= effective_y && current_y < effective_y.wrapping_add(sprite_height) {
            // This sprite intersects the current scanline

            if visible_sprites.len() < 10 { // Hardware limit: max 10 sprites per scanline
                 let tile_index = memory_bus.read_byte(oam_addr + 2);
                 let attributes = memory_bus.read_byte(oam_addr + 3);

                 let palette_num = (attributes >> OAM_PALETTE_NUM_DMG) & 1;
                 let palette_reg_value = if palette_num == 0 { obp0 } else { obp1 };

                 visible_sprites.push(SpriteInfo {
                    oam_index: i as u8,
                    y_pos: sprite_y,
                    x_pos: sprite_x,
                    tile_index,
                    attributes,
                    height: sprite_height,
                    palette_reg_value,
                    x_flip: (attributes & (1 << OAM_X_FLIP)) != 0,
                    y_flip: (attributes & (1 << OAM_Y_FLIP)) != 0,
                    bg_priority: (attributes & (1 << OAM_BG_WIN_PRIORITY)) != 0,
                });
            } else {
                 break; // Stop searching once 10 sprites are found
            }
        }
    }

    // Sort the found sprites by X-coordinate (ascending), then OAM index (ascending)
    // This ensures correct rendering priority for overlapping sprites at the same X.
    visible_sprites.sort_unstable_by(|a, b| {
        a.x_pos.cmp(&b.x_pos).then_with(|| a.oam_index.cmp(&b.oam_index))
    });

    visible_sprites
}


/// Calculates the pixel index (0-3) within a specific sprite's tile data.
#[inline]
fn get_sprite_tile_pixel_index(
    sprite: &SpriteInfo,
    col_in_tile: u8, // Column within the 8x8 pattern (0-7, already adjusted for x-flip)
    memory_bus: &MemoryBus,
) -> u8 {

    // Calculate the row within the tile pattern (adjusting for y-flip and height)
    let current_y = memory_bus.read_byte(memory_map::LY_ADDR); // Read LY for current scanline
    let effective_y = sprite.y_pos.wrapping_sub(16); // Screen Y coordinate of top edge
    let mut row_in_sprite = current_y - effective_y; // Row within the full sprite height (0-7 or 0-15)

    if sprite.y_flip {
        row_in_sprite = (sprite.height - 1) - row_in_sprite;
    }

    // Determine the actual tile index and adjust row for 8x16 sprites
    let actual_tile_index = if sprite.height == 16 {
        if row_in_sprite < 8 { sprite.tile_index & 0xFE } else { sprite.tile_index | 0x01 }
    } else {
        sprite.tile_index
    };
    let row_in_tile = row_in_sprite % 8; // Row within the 8x8 tile pattern (0-7)

    // Sprites always use $8000-$8FFF tile data area
    let tile_addr = memory_map::VRAM_START + (actual_tile_index as u16 * 16);
    let row_addr = tile_addr + (row_in_tile as u16 * 2);

    // Read the row data and extract pixel index
    get_tile_row_pixel_index(row_addr, col_in_tile, memory_bus)
}