use crate::memory_bus::MemoryBus;

// --- Constants --- (Keep as before)
const CPU_FREQ: u32 = 4_194_304;
const FRAME_SEQUENCER_FREQ_HZ: u32 = 512;
const FRAME_SEQUENCER_DIVIDER: u32 = CPU_FREQ / FRAME_SEQUENCER_FREQ_HZ;

const NR10_ADDR: u16 = 0xFF10; // Channel 1 Sweep
const NR11_ADDR: u16 = 0xFF11; // Channel 1 Length/Duty
const NR12_ADDR: u16 = 0xFF12; // Channel 1 Volume/Envelope
const NR13_ADDR: u16 = 0xFF13; // Channel 1 Frequency Lo (Write-only)
const NR14_ADDR: u16 = 0xFF14; // Channel 1 Frequency Hi/Control

const NR21_ADDR: u16 = 0xFF16; // Channel 2 Length/Duty
const NR22_ADDR: u16 = 0xFF17; // Channel 2 Volume/Envelope
const NR23_ADDR: u16 = 0xFF18; // Channel 2 Frequency Lo (Write-only)
const NR24_ADDR: u16 = 0xFF19; // Channel 2 Frequency Hi/Control

const NR30_ADDR: u16 = 0xFF1A; // Channel 3 DAC Enable
const NR31_ADDR: u16 = 0xFF1B; // Channel 3 Length (Write-only)
const NR32_ADDR: u16 = 0xFF1C; // Channel 3 Volume Level
const NR33_ADDR: u16 = 0xFF1D; // Channel 3 Frequency Lo (Write-only)
const NR34_ADDR: u16 = 0xFF1E; // Channel 3 Frequency Hi/Control
const WAVE_RAM_START: u16 = 0xFF30;
const WAVE_RAM_END: u16 = 0xFF3F;

const NR41_ADDR: u16 = 0xFF20; // Channel 4 Length (Write-only)
const NR42_ADDR: u16 = 0xFF21; // Channel 4 Volume/Envelope
const NR43_ADDR: u16 = 0xFF22; // Channel 4 Clock Shift/Width/Divisor
const NR44_ADDR: u16 = 0xFF23; // Channel 4 Control

const NR50_ADDR: u16 = 0xFF24; // Master Volume / VIN Panning
const NR51_ADDR: u16 = 0xFF25; // Sound Panning
const NR52_ADDR: u16 = 0xFF26; // Sound On/Off (Status)

// Default power-on register values (from PanDocs or common emulators)
const NR10_DEFAULT: u8 = 0x80;
const NR11_DEFAULT: u8 = 0xBF; // Duty readable (bits 6-7), Length write-only
const NR12_DEFAULT: u8 = 0xF3; // Need to check exact power-on state for volume=0? Often F3 means DAC off
const NR14_DEFAULT: u8 = 0xBF; // Length enable readable (bit 6)
const NR21_DEFAULT: u8 = 0x3F; // Duty readable (bits 6-7), Length write-only
const NR22_DEFAULT: u8 = 0x00;
const NR24_DEFAULT: u8 = 0xBF; // Length enable readable (bit 6)
const NR30_DEFAULT: u8 = 0x7F; // DAC enable readable (bit 7)
const NR32_DEFAULT: u8 = 0x9F; // Volume readable (bits 5-6)
const NR34_DEFAULT: u8 = 0xBF; // Length enable readable (bit 6)
const NR42_DEFAULT: u8 = 0x00;
const NR43_DEFAULT: u8 = 0x00;
const NR44_DEFAULT: u8 = 0xBF; // Length enable readable (bit 6)
const NR50_DEFAULT: u8 = 0x77; // Check PanDocs (seems to vary slightly)
const NR51_DEFAULT: u8 = 0xF3; // Check PanDocs
// NR52 is status/control, read constructed, write only affects bit 7

pub struct Apu {
    // --- Timing ---
    cycle_counter: u32,
    frame_sequencer_step: u8,

    // --- Master Control ---
    apu_enabled: bool,
    // nr50/nr51 are mirrored directly
    nr50: u8,
    nr51: u8,

    // --- Mirrored Register Values ---
    // These store the last value written via write_byte.
    // read_byte uses these + masks for read-only bits.
    nr10: u8,
    nr11: u8,
    nr12: u8,
    // nr13 is write-only
    nr14: u8,
    // nr20 doesn't exist
    nr21: u8,
    nr22: u8,
    // nr23 is write-only
    nr24: u8,
    nr30: u8,
    // nr31 is write-only
    nr32: u8,
    // nr33 is write-only
    nr34: u8,
    // nr41 is write-only
    nr42: u8,
    nr43: u8,
    nr44: u8,

    // --- Channel State (Placeholders - requires detailed implementation) ---
    // TODO: Replace these with actual channel state structs
    ch1_active: bool,
    ch2_active: bool,
    ch3_active: bool,
    ch4_active: bool,
    // Add full state for each channel here...
}

impl Apu {
    pub fn new() -> Self {
        Apu {
            cycle_counter: 0,
            frame_sequencer_step: 0,

            apu_enabled: false,
            nr50: NR50_DEFAULT,
            nr51: NR51_DEFAULT,

            // Initialize mirrored registers to power-on defaults
            nr10: NR10_DEFAULT,
            nr11: NR11_DEFAULT,
            nr12: NR12_DEFAULT,
            nr14: NR14_DEFAULT,
            nr21: NR21_DEFAULT,
            nr22: NR22_DEFAULT,
            nr24: NR24_DEFAULT,
            nr30: NR30_DEFAULT,
            nr32: NR32_DEFAULT,
            nr34: NR34_DEFAULT,
            nr42: NR42_DEFAULT,
            nr43: NR43_DEFAULT,
            nr44: NR44_DEFAULT,

            // Initialize channel states (placeholders)
            ch1_active: false,
            ch2_active: false,
            ch3_active: false,
            ch4_active: false,
            // Initialize full channel state structs...
        }
    }

    pub fn step(&mut self, cycles: u32, memory_bus: &mut MemoryBus) {
        if !self.apu_enabled {
            return;
        }

        // --- Frame Sequencer Clocking ---
        self.cycle_counter += cycles;
        while self.cycle_counter >= FRAME_SEQUENCER_DIVIDER {
            self.cycle_counter -= FRAME_SEQUENCER_DIVIDER;
            match self.frame_sequencer_step {
                0 => self.clock_length_counters(memory_bus),
                1 => { /* Nothing */ }
                2 => {
                    self.clock_length_counters(memory_bus);
                    self.clock_sweep_unit(memory_bus);
                }
                3 => { /* Nothing */ }
                4 => self.clock_length_counters(memory_bus),
                5 => { /* Nothing */ }
                6 => {
                    self.clock_length_counters(memory_bus);
                    self.clock_sweep_unit(memory_bus);
                }
                7 => self.clock_envelope_units(memory_bus),
                _ => unreachable!(),
            }
            self.frame_sequencer_step = (self.frame_sequencer_step + 1) % 8;
        }

        // --- Channel Frequency Timers ---
        // TODO: Clock individual channel frequency timers based on `cycles`.

        // --- Sample Generation ---
        // TODO: Generate audio samples at the target output rate.
    }

    /// Reads a byte from an APU register address (0xFF10-0xFF26).
    /// Uses internally mirrored values + masks for read-only bits.
    /// Does NOT typically call memory_bus.read_byte to avoid side effects.
    pub fn read_byte(&self, addr: u16) -> u8 {
        // Note: No MemoryBus argument needed here anymore for reads,
        // as we use the internal mirrored state.

        match addr {
            // --- Channel 1: Pulse A (Sweep) ---
            NR10_ADDR => self.nr10 | 0x80, // Bits 0-6 readable, bit 7 often reads 1
            NR11_ADDR => self.nr11 | 0x3F, // Only bits 6-7 (Duty) readable
            NR12_ADDR => self.nr12,
            NR13_ADDR => 0xFF,             // Write-only
            NR14_ADDR => self.nr14 | 0xBF, // Only bit 6 (Length Enable) readable

            // --- Channel 2: Pulse B ---
            // NR20 does not exist
            NR21_ADDR => self.nr21 | 0x3F, // Only bits 6-7 (Duty) readable
            NR22_ADDR => self.nr22,
            NR23_ADDR => 0xFF,             // Write-only
            NR24_ADDR => self.nr24 | 0xBF, // Only bit 6 (Length Enable) readable

            // --- Channel 3: Wave ---
            NR30_ADDR => self.nr30 | 0x7F, // Only bit 7 (DAC Enable) readable
            NR31_ADDR => 0xFF,             // Write-only (Length)
            NR32_ADDR => self.nr32 | 0x9F, // Only bits 5-6 (Volume) readable
            NR33_ADDR => 0xFF,             // Write-only (Freq Lo)
            NR34_ADDR => self.nr34 | 0xBF, // Only bit 6 (Length Enable) readable

            // --- Channel 4: Noise ---
            // NR40 does not exist
            NR41_ADDR => 0xFF, // Write-only (Length)
            NR42_ADDR => self.nr42,
            NR43_ADDR => self.nr43,
            NR44_ADDR => self.nr44 | 0xBF, // Only bit 6 (Length Enable) readable

            // --- Master Control ---
            NR50_ADDR => self.nr50,
            NR51_ADDR => self.nr51,
            NR52_ADDR => {
                // Construct NR52 dynamically based on internal state
                let mut nr52 = 0u8;
                if self.apu_enabled {
                    nr52 |= 0x80;
                }
                // TODO: Update placeholder active flags from real channel state
                if self.ch1_active {
                    nr52 |= 0x01;
                }
                if self.ch2_active {
                    nr52 |= 0x02;
                }
                if self.ch3_active {
                    nr52 |= 0x04;
                }
                if self.ch4_active {
                    nr52 |= 0x08;
                }
                nr52 | 0x70 // Bits 4-6 read as 1
            }

            _ => {
                // Return 0xFF for unused registers in APU range (e.g., FF15, FF1F, etc.)
                // eprintln!("Warning: Unhandled APU read at {:04X}", addr);
                0xFF
            }
        }
    }

    /// Writes a byte to an APU register address (0xFF10-0xFF26).
    /// Updates internal mirrored state and triggers APU actions.
    pub fn write_byte(&mut self, addr: u16, value: u8, memory_bus: &mut MemoryBus) {
        // --- Handle NR52 Master Control Write FIRST ---
        if addr == NR52_ADDR {
            let previous_enabled_state = self.apu_enabled;
            self.apu_enabled = (value & 0x80) != 0;
            if previous_enabled_state && !self.apu_enabled {
                self.reset_apu_state_and_registers(memory_bus);
            }
            // Don't update mirrored value for NR52; read is dynamic
            return;
        }

        // --- If APU is disabled, most register writes are blocked ---
        if !self.apu_enabled {
            if !(WAVE_RAM_START..=WAVE_RAM_END).contains(&addr) {
                // Block register writes if APU is off (except Wave RAM, handled by bus)
                return;
            }
        }

        // --- Handle writes to specific registers ---
        // Update mirrored state *before* handling side effects for simplicity here.
        // In a real HW impl, side effects might read the *new* value.
        match addr {
            // --- Channel 1 ---
            NR10_ADDR => {
                self.nr10 = value; /* TODO: Update sweep state */
            }
            NR11_ADDR => {
                self.nr11 = value; /* TODO: Update length timer (bits 0-5), duty cycle (bits 6-7) */
            }
            NR12_ADDR => {
                self.nr12 = value; /* TODO: Update envelope state, check DAC power */
            }
            NR13_ADDR => { /* Write-only, update internal freq state */ }
            NR14_ADDR => {
                self.nr14 = value; /* TODO: Update freq hi, handle TRIGGER(7), update length enable(6) */
            }

            // --- Channel 2 ---
            NR21_ADDR => {
                self.nr21 = value; /* TODO: Update length timer, duty cycle */
            }
            NR22_ADDR => {
                self.nr22 = value; /* TODO: Update envelope state, check DAC power */
            }
            NR23_ADDR => { /* Write-only, update internal freq state */ }
            NR24_ADDR => {
                self.nr24 = value; /* TODO: Update freq hi, handle TRIGGER(7), update length enable(6) */
            }

            // --- Channel 3 ---
            NR30_ADDR => {
                self.nr30 = value; /* TODO: Update DAC enable (bit 7) */
            }
            NR31_ADDR => { /* Write-only, update internal length state */ }
            NR32_ADDR => {
                self.nr32 = value; /* TODO: Update volume level (bits 5-6) */
            }
            NR33_ADDR => { /* Write-only, update internal freq state */ }
            NR34_ADDR => {
                self.nr34 = value; /* TODO: Update freq hi, handle TRIGGER(7), update length enable(6) */
            }

            // --- Channel 4 ---
            NR41_ADDR => { /* Write-only, update internal length state */ }
            NR42_ADDR => {
                self.nr42 = value; /* TODO: Update envelope state, check DAC power */
            }
            NR43_ADDR => {
                self.nr43 = value; /* TODO: Update clock shift, width mode, dividing ratio */
            }
            NR44_ADDR => {
                self.nr44 = value; /* TODO: Handle TRIGGER(7), update length enable(6) */
            }

            // --- Master Control ---
            NR50_ADDR => self.nr50 = value,
            NR51_ADDR => self.nr51 = value,

            _ => { /* Ignore writes to unused/read-only/NR52 here */ }
        }
    }

    // --- Helper Functions for Frame Sequencer (Keep placeholders) ---
    fn clock_length_counters(&mut self, _memory_bus: &MemoryBus) { /* TODO */
    }
    fn clock_sweep_unit(&mut self, _memory_bus: &MemoryBus) { /* TODO */
    }
    fn clock_envelope_units(&mut self, _memory_bus: &MemoryBus) { /* TODO */
    }

    /// Resets APU registers (mirrored state) and internal state when NR52 bit 7 is written to 0.
    fn reset_apu_state_and_registers(&mut self, memory_bus: &mut MemoryBus) {
        println!("APU Disabled: Resetting state and registers (Partial Implementation)");

        // Reset internal timing
        self.cycle_counter = 0;
        self.frame_sequencer_step = 0;

        // Reset mirrored registers to defaults
        self.nr10 = NR10_DEFAULT;
        self.nr11 = NR11_DEFAULT;
        self.nr12 = NR12_DEFAULT;
        self.nr14 = NR14_DEFAULT;
        self.nr21 = NR21_DEFAULT;
        self.nr22 = NR22_DEFAULT;
        self.nr24 = NR24_DEFAULT;
        self.nr30 = NR30_DEFAULT;
        self.nr32 = NR32_DEFAULT;
        self.nr34 = NR34_DEFAULT;
        self.nr42 = NR42_DEFAULT;
        self.nr43 = NR43_DEFAULT;
        self.nr44 = NR44_DEFAULT;
        // NR50/NR51 might not reset? Check PanDocs. Keep current values or reset?
        // self.nr50 = NR50_DEFAULT;
        // self.nr51 = NR51_DEFAULT;

        // TODO: Reset *all* internal channel states (timers, counters, volume, freq, etc.)
        self.ch1_active = false;
        self.ch2_active = false;
        self.ch3_active = false;
        self.ch4_active = false;

        // Clear actual registers on the bus *if required* by hardware spec
        // This ensures consistency if other components read the bus directly.
        // Use `write_byte_direct` if available in MemoryBus to bypass APU logic.
        // memory_bus.write_byte_direct(NR10_ADDR, self.nr10);
        // memory_bus.write_byte_direct(NR11_ADDR, self.nr11);
        // ... etc for all writable registers ...
        // memory_bus.write_byte_direct(NR50_ADDR, self.nr50);
        // memory_bus.write_byte_direct(NR51_ADDR, self.nr51);
    }

    // --- TODO: Add Sample Generation Logic ---
    // --- TODO: Add detailed channel state structs and methods ---
}
