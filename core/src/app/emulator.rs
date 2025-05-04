use crate::constants;
use boba::apu::Apu;
use boba::cpu::Cpu; // Use Cpu from lib
use boba::memory_bus::MemoryBus;
use boba::ppu::Ppu; // Use Ppu from lib
use std::fs;
use std::path::Path; // Use constants from sibling module

/// Represents the core Game Boy emulator components.
pub struct Emulator {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub apu: Apu,
    pub memory_bus: MemoryBus,
    /// Flag to indicate if the emulator is currently paused for instruction stepping.
    pub stepping: bool, // Added for stepping mode
}

impl Emulator {
    /// Creates a new Emulator instance, loads the ROM, and initializes components.
    pub fn new(rom_path: &Path, skip_boot_rom: bool) -> Result<Self, String> {
        println!("Initializing APU...");
        let apu = Apu::new(); // Assuming Apu::new() exists

        println!("Initializing memory bus...");
        let mut memory_bus = MemoryBus::new(); // Assuming MemoryBus::new() exists

        println!("Loading ROM: {}", rom_path.display());
        let rom_data = fs::read(rom_path)
            .map_err(|e| format!("Failed to read ROM '{}': {}", rom_path.display(), e))?;
        let rom_size = rom_data.len();
        memory_bus.load_rom(&rom_data);
        println!("ROM loaded successfully ({} bytes)", rom_size);

        println!("Initializing CPU (skip_boot_rom={})...", skip_boot_rom);
        // NOTE: Kept `let mut cpu` as in your original code snippet.
        // If `cpu` isn't modified later in `new`, `let cpu` might be more conventional.
        let mut cpu = Cpu::new(skip_boot_rom); // Assuming Cpu::new(bool) exists

        if skip_boot_rom {
            println!("Skipping boot ROM - initializing I/O registers post-boot...");
            // Assuming Cpu::initialize_post_boot_io() exists and takes &mut MemoryBus
            Cpu::initialize_post_boot_io(&mut memory_bus);
        }
        // Note: The 'else' case (loading and running boot ROM) is not handled here,
        // matching the provided code.

        println!("Initializing PPU...");
        let ppu = Ppu::new(); // Assuming Ppu::new() exists

        Ok(Emulator {
            cpu,
            ppu,
            apu,
            memory_bus,
            stepping: true, // Initialize stepping mode to off (continuous run)
        })
    }

    /// Toggles the emulator between continuous run mode and single-step mode.
    pub fn toggle_stepping(&mut self) {
        self.stepping = !self.stepping;
        if self.stepping {
            println!("[Stepping Mode Enabled]");
        } else {
            println!("[Continuous Mode Enabled]");
        }
    }

    /// Executes exactly one CPU instruction and updates PPU/APU accordingly.
    /// Use this when `stepping` mode is enabled.
    /// Returns `Ok(())` or an error string if the CPU encounters an error.
    pub fn step_instruction(&mut self) -> Result<(), String> {
        // 1. Step CPU - returns cycles executed or error
        let executed_cycles = match self.cpu.step(&mut self.memory_bus) {
            Ok(cycles) => cycles as u32,
            Err(error_message) => {
                // Propagate the CPU error immediately, even in stepping mode
                return Err(error_message);
            }
        };

        // Optional: Print CPU state after step for debugging
        // println!("Stepped: PC=${:04X} Cycles: {} Flags: {:08b}", self.cpu.registers.pc, executed_cycles, self.cpu.registers.f);

        // 2. Step PPU with the cycles the CPU used
        self.ppu.step(executed_cycles, &mut self.memory_bus);

        // 3. Step APU with the cycles the CPU used
        self.apu.step(executed_cycles, &mut self.memory_bus); // Assuming apu.step exists

        Ok(())
    }

    /// Runs the emulator components for approximately one frame's worth of CPU cycles.
    /// If `stepping` mode is enabled, this function does nothing immediately.
    /// Returns `Ok(())` or an error string if the CPU encounters an error during the frame.
    pub fn run_frame(&mut self) -> Result<(), String> {
        // If stepping is enabled, don't run the frame automatically.
        // Execution must be triggered via step_instruction().
        if self.stepping {
            return Ok(());
        }

        // --- Continuous Execution Logic (only runs if self.stepping is false) ---
        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < constants::CYCLES_PER_FRAME {
            // 1. Step CPU - returns cycles executed or error
            let executed_cycles = match self.cpu.step(&mut self.memory_bus) {
                Ok(cycles) => cycles as u32,
                Err(error_message) => {
                    // Propagate the CPU error immediately
                    return Err(error_message);
                }
            };

            // 2. Step PPU with the cycles the CPU used
            self.ppu.step(executed_cycles, &mut self.memory_bus);

            // 3. Step APU with the cycles the CPU used
            self.apu.step(executed_cycles, &mut self.memory_bus); // Assuming apu.step exists

            // 4. Accumulate cycles
            cycles_this_frame += executed_cycles;
        }
        Ok(()) // Frame completed successfully
    }
}
