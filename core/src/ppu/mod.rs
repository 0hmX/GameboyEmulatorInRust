use crate::memory_bus::MemoryBus;
use crate::memory_map; // Use memory_map constants directly

mod constants;
mod state;
mod render;
mod debug;

// Re-export public constants and types
pub use constants::{GB_WIDTH, GB_HEIGHT, VRAM_DEBUG_WIDTH, VRAM_DEBUG_HEIGHT};
use constants::*; // Use internal constants
use state::PpuState;

/// Represents the Picture Processing Unit (PPU) of the Game Boy.
pub struct Ppu {
    frame_buffer: Box<[u8; FRAME_BUFFER_SIZE]>, // Use Box for heap allocation
    vram_debug_buffer: Box<[u8; VRAM_DEBUG_BUFFER_SIZE]>, // Use Box for heap allocation
    state: PpuState,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            frame_buffer: Box::new([0; FRAME_BUFFER_SIZE]),
            vram_debug_buffer: Box::new([0; VRAM_DEBUG_BUFFER_SIZE]),
            state: PpuState::new(),
        }
    }

    /// Get a reference to the current Game Boy screen frame buffer.
    pub fn get_frame_buffer(&self) -> &[u8; FRAME_BUFFER_SIZE] {
        &self.frame_buffer
    }

    /// Get a reference to the VRAM debug view buffer.
    pub fn get_vram_debug_buffer(&self) -> &[u8; VRAM_DEBUG_BUFFER_SIZE] {
        &self.vram_debug_buffer
    }

    /// Call this periodically (e.g., once per frame) to update the VRAM debug view.
    pub fn update_vram_debug_buffer(&mut self, memory_bus: &MemoryBus) {
        debug::render_vram_debug(&mut self.vram_debug_buffer, memory_bus);
    }

    /// Steps the PPU by the given number of T-cycles. Handles timing, mode transitions,
    /// rendering, and interrupt requests.
    pub fn step(&mut self, cycles: u32, memory_bus: &mut MemoryBus) {
        // --- Read LCDC and STAT ---
        // Caching these helps avoid frequent bus reads within the step logic.
        self.state.lcdc = memory_bus.read_byte(memory_map::LCDC_ADDR);
        self.state.stat = memory_bus.read_byte(memory_map::STAT_ADDR);

        // --- Check if LCD is enabled ---
        if (self.state.lcdc & (1 << LCDC_LCD_ENABLE)) == 0 {
            // LCD is off - reset state if not already reset
            if self.state.dots != 0 || self.state.current_scanline != 0 || self.state.ppu_mode != HBLANK_MODE {
                self.state.reset_for_lcd_off();
                // Write initial state to registers when LCD turns off
                memory_bus.write_byte(memory_map::LY_ADDR, 0);
                 // Preserve IE bits, force mode to 0 (HBLANK), clear coincidence flag
                let stat_to_write = (self.state.stat & 0b1111_1000) | HBLANK_MODE;
                memory_bus.set_io_reg_direct(memory_map::STAT_ADDR, stat_to_write); // Use direct write if available to bypass PPU write checks

                // Clear frame buffer? Optional. Some games rely on VRAM content.
                // self.frame_buffer.fill(0);
            }
            return; // Do nothing else if LCD is off
        }

        // --- Advance PPU timing ---
        self.state.dots += cycles;

        // --- State transitions based on timing ---
        match self.state.ppu_mode {
            OAM_SCAN_MODE => { // Mode 2
                if self.state.dots >= MODE2_OAM_SCAN_DOTS {
                    self.state.dots -= MODE2_OAM_SCAN_DOTS;
                    self.state.ppu_mode = VRAM_READ_MODE; // Transition to Mode 3
                }
            }
            VRAM_READ_MODE => { // Mode 3
                 // Mode 3 duration can vary slightly based on sprites, etc.
                 // For simplicity, use the minimum duration. A more accurate PPU
                 // might calculate the exact end point.
                 if self.state.dots >= MODE3_VRAM_READ_DOTS {
                    self.state.dots -= MODE3_VRAM_READ_DOTS;
                    self.state.ppu_mode = HBLANK_MODE; // Transition to Mode 0

                    // --- Render the scanline just before entering HBlank ---
                    let y = self.state.current_scanline as usize;
                    if y < GB_HEIGHT {
                        let start_index = y * GB_WIDTH;
                        let end_index = start_index + GB_WIDTH;
                        // Get a mutable slice for the current line
                        let line_buffer_slice = &mut self.frame_buffer[start_index..end_index];
                        // Convert slice to array reference (requires exact size match)
                         if let Ok(line_buffer_array) = line_buffer_slice.try_into() {
                             render::render_scanline(line_buffer_array, &self.state, memory_bus);
                         } else {
                             // Handle error: slice length didn't match array size (shouldn't happen here)
                             log::error!("Failed to get line buffer slice for rendering!");
                         }
                    }
                }
            }
            HBLANK_MODE => { // Mode 0
                // Mode 0 ends when the total dots for the scanline are reached
                if self.state.dots >= DOTS_PER_SCANLINE {
                    self.state.dots %= DOTS_PER_SCANLINE; // Keep leftover dots for next line
                    self.state.current_scanline += 1;

                    // Check for end of visible frame -> VBlank start
                    if self.state.current_scanline == GB_HEIGHT as u8 {
                        self.state.ppu_mode = VBLANK_MODE; // Transition to Mode 1
                        self.state.vblank_just_occurred = true; // Signal VBlank interrupt
                        // Option: Render VRAM debug view once per frame here
                        // self.update_vram_debug_buffer(memory_bus);
                    } else {
                        // Start next visible line
                        self.state.ppu_mode = OAM_SCAN_MODE; // Transition back to Mode 2
                    }
                    // Update LY register *after* potential mode change for the new line
                    memory_bus.set_io_reg_direct(memory_map::LY_ADDR, self.state.current_scanline);
                }
            }
            VBLANK_MODE => { // Mode 1
                 if self.state.dots >= DOTS_PER_SCANLINE {
                    self.state.dots %= DOTS_PER_SCANLINE;
                    self.state.current_scanline += 1;

                    // Check for end of VBlank -> frame wrap
                    if self.state.current_scanline == SCANLINES_PER_FRAME {
                        self.state.current_scanline = 0; // Wrap back to line 0
                        self.state.ppu_mode = OAM_SCAN_MODE; // Start frame over in Mode 2
                    }
                     // Always update LY during VBlank
                    memory_bus.set_io_reg_direct(memory_map::LY_ADDR, self.state.current_scanline);
                }
            }
            _ => unreachable!("Invalid PPU mode: {}", self.state.ppu_mode),
        }

        // --- Update LYC=LY Flag and STAT Register ---
        self.check_lyc_coincidence(memory_bus);
        self.update_stat_register(memory_bus);

        // --- Handle Interrupt Requests ---
        self.check_and_request_interrupts(memory_bus);
    }


    /// Checks LYC=LY coincidence and updates the internal flag.
    fn check_lyc_coincidence(&mut self, memory_bus: &MemoryBus) {
        let lyc = memory_bus.read_byte(memory_map::LYC_ADDR);
        self.state.lyc_eq_ly = self.state.current_scanline == lyc;
    }

    /// Updates the STAT register on the memory bus based on current PPU state.
    fn update_stat_register(&mut self, memory_bus: &mut MemoryBus) {
        // Preserve the writable bits (interrupt enables) from the cached STAT value
        let writable_bits = self.state.stat & 0b0111_1000;
        // Combine with the current mode and coincidence flag (read-only bits)
        let mut new_stat = writable_bits | self.state.ppu_mode;
        if self.state.lyc_eq_ly {
            new_stat |= 1 << STAT_LYC_EQ_LY_FLAG;
        }
         // Bit 7 is always set (unused, reads as 1)
        new_stat |= 0x80;

        // Write the updated STAT value (only if changed?)
        // Using direct write to avoid infinite loops if STAT write itself triggers PPU logic.
        memory_bus.set_io_reg_direct(memory_map::STAT_ADDR, new_stat);
    }


    /// Checks conditions for STAT and VBlank interrupts and requests them.
    fn check_and_request_interrupts(&mut self, memory_bus: &mut MemoryBus) {
        // --- Check VBlank Interrupt ---
        if self.state.vblank_just_occurred {
            Self::request_interrupt(memory_bus, memory_map::VBLANK_INTERRUPT_BIT);
            self.state.vblank_just_occurred = false;
        }

        // --- Check STAT Interrupt Conditions ---
        // Use the *cached* STAT register value read at the start of step()
        // to check the *enabled* interrupts.
        let stat_reg = self.state.stat;
        let mut stat_interrupt_now = false;

        // LYC=LY interrupt enabled and condition met?
        if (stat_reg & (1 << STAT_LYC_EQ_LY_IE)) != 0 && self.state.lyc_eq_ly {
            stat_interrupt_now = true;
        }
        // Mode 0 HBlank interrupt enabled and currently in Mode 0?
        if (stat_reg & (1 << STAT_MODE_0_HBLANK_IE)) != 0 && self.state.ppu_mode == HBLANK_MODE {
            stat_interrupt_now = true;
        }
        // Mode 1 VBlank interrupt enabled and currently in Mode 1?
        if (stat_reg & (1 << STAT_MODE_1_VBLANK_IE)) != 0 && self.state.ppu_mode == VBLANK_MODE {
            stat_interrupt_now = true;
        }
        // Mode 2 OAM interrupt enabled and currently in Mode 2?
        if (stat_reg & (1 << STAT_MODE_2_OAM_IE)) != 0 && self.state.ppu_mode == OAM_SCAN_MODE {
            stat_interrupt_now = true;
        }

        // Request STAT interrupt only on the rising edge (when the condition *becomes* true)
        if stat_interrupt_now && !self.state.stat_interrupt_line {
            Self::request_interrupt(memory_bus, memory_map::LCD_STAT_INTERRUPT_BIT);
        }
        // Update the internal state of the STAT interrupt line for the next cycle's check
        self.state.stat_interrupt_line = stat_interrupt_now;
    }


    /// Helper to request an interrupt by setting the corresponding bit in the IF register.
    #[inline]
    fn request_interrupt(memory_bus: &mut MemoryBus, bit: u8) {
        let current_if = memory_bus.read_byte(memory_map::IF_ADDR);
        // Use direct write to avoid potential side effects of a normal write_byte
        memory_bus.set_io_reg_direct(memory_map::IF_ADDR, current_if | (1 << bit));
    }
}