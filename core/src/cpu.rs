use crate::memory_bus::MemoryBus; // Assuming memory_bus.rs is in the same crate src directory

// Define bit positions within the F register
const FLAG_Z_POS: u8 = 7; // Zero flag
const FLAG_N_POS: u8 = 6; // Subtract flag (BCD)
const FLAG_H_POS: u8 = 5; // Half Carry flag (BCD)
const FLAG_C_POS: u8 = 4; // Carry flag

// Masks for convenient flag setting/clearing
const FLAG_Z: u8 = 1 << FLAG_Z_POS;
const FLAG_N: u8 = 1 << FLAG_N_POS;
const FLAG_H: u8 = 1 << FLAG_H_POS;
const FLAG_C: u8 = 1 << FLAG_C_POS;

/// Represents the Game Boy CPU (Sharp LR35902).
pub struct Cpu<'a> {
    // Registers
    a: u8, // Accumulator
    f: u8, // Flags register
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,

    sp: u16, // Stack Pointer
    pc: u16, // Program Counter

    memory_bus: &'a mut MemoryBus,

    ime: bool,        // Interrupt Master Enable flag (true=enabled, false=disabled)
    halted: bool,     // Is the CPU currently halted?
    total_cycles: u64 // For tracking total cycles executed (optional but useful)
}

impl<'a> Cpu<'a> {
    pub fn new(memory_bus: &'a mut MemoryBus, has_boot_rom: bool) -> Self {
        Cpu {
            a: if has_boot_rom { 0x00 } else { 0x01 },  // 0x01 DMG/0x11 CGB post-boot
            f: if has_boot_rom { 0x00 } else { 0xB0 },  // Flags reset during boot
            b: 0x00,
            c: 0x00,  // Typically 0x00 at boot, set to 0x13 by boot ROM
            d: 0x00,
            e: 0x00,  // Typically 0x00 at boot, set to 0xD8 by boot ROM
            h: 0x00,  // Typically 0x00 at boot, set to 0x01 by boot ROM
            l: 0x00,  // Typically 0x00 at boot, set to 0x4D by boot ROM
            sp: 0xFFFE,  // Stack pointer constant
            pc: if has_boot_rom { 0x0000 } else { 0x0100 },  // Boot ROM vs game entry

            memory_bus,
            ime: false,
            halted: false,
            total_cycles: 0,
        }
    }

    /// Executes a single CPU instruction cycle (fetch, decode, execute).
    /// Returns the number of **T-cycles** (clock cycles) the instruction took.
    pub fn step(&mut self) -> u8 {
        // --- 1. Handle HALT state ---
        if self.halted {
             // Check for pending interrupts that could wake the CPU
             // TODO: Implement interrupt checking here
             // If woken by interrupt: self.halted = false;
             // If still halted, just consume time (1 M-cycle / 4 T-cycles)
             self.total_cycles += 4;
             return 4; // NOP duration
        }

        // --- 2. Check for and Handle Interrupts (if IME is enabled) ---
        // TODO: Implement interrupt handling logic here
        //       - Check IF register (memory 0xFF0F) & IE register (memory 0xFFFF)
        //       - If interrupt pending and enabled:
        //         - Reset IME
        //         - Push PC onto stack
        //         - Jump to interrupt handler vector
        //         - Consume extra cycles
        //       - If an interrupt was handled, return the cycle count for the handler

        // --- 3. Fetch ---
        let opcode = self.fetch_byte();

        // --- 4. Decode & Execute ---
        let cycles = self.execute(opcode);
        self.total_cycles += cycles as u64;

        // Return T-cycles for this instruction
        cycles
    }

    /// Fetches the next byte from memory at the PC and increments the PC.
    fn fetch_byte(&mut self) -> u8 {
        let byte = self.memory_bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    /// Fetches the next word (16 bits, little-endian) from memory at the PC
    /// and increments the PC by 2.
    fn fetch_word(&mut self) -> u16 {
        let low = self.fetch_byte() as u16;
        let high = self.fetch_byte() as u16;
        (high << 8) | low
    }


    /// Decodes and executes a fetched opcode.
    /// Returns the number of T-cycles the instruction took.
    fn execute(&mut self, opcode: u8) -> u8 {
        match opcode {
            // --- NOP ---
            0x00 => {
                // No operation
                4 // 1 M-cycle * 4 T-cycles/M-cycle
            }

            // --- LD A, n8 ---
            0x3E => {
                let value = self.fetch_byte();
                self.a = value;
                8 // 2 M-cycles
            }

            // --- XOR A ---
            // XORs register A with itself. Result is always 0.
            0xAF => {
                self.a ^= self.a; // Sets A to 0
                self.set_flag(FLAG_Z, self.a == 0); // Set Z flag
                self.set_flag(FLAG_N, false);       // Clear N flag
                self.set_flag(FLAG_H, false);       // Clear H flag
                self.set_flag(FLAG_C, false);       // Clear C flag
                4 // 1 M-cycle
            }

            // --- JP nn ---
            // Unconditional jump to 16-bit immediate address nn
            0xC3 => {
                let jump_addr = self.fetch_word();
                self.pc = jump_addr;
                16 // 4 M-cycles (fetch opcode + fetch word)
            }

            // --- LD (HL), A ---
            // Store value of register A into memory location pointed to by HL
            0x77 => {
                self.memory_bus.write_byte(self.get_hl(), self.a);
                8 // 2 M-cycles
            }

            // --- LD A, (HL) ---
            // Load value from memory location pointed to by HL into register A
            0x7E => {
                 self.a = self.memory_bus.read_byte(self.get_hl());
                 8 // 2 M-cycles
            }

            // --- INC HL ---
            // Increment 16-bit register HL
            0x23 => {
                let hl = self.get_hl().wrapping_add(1);
                self.set_hl(hl);
                8 // 2 M-cycles (internal operation)
            }

            // --- JR NZ, r8 ---
            // Relative jump by signed 8-bit value if Zero Flag is not set
            0x20 => {
                let relative_offset = self.fetch_byte() as i8; // Read signed offset
                if !self.get_flag(FLAG_Z) {
                    // Jump taken
                    self.pc = self.pc.wrapping_add(relative_offset as u16);
                    12 // 3 M-cycles
                } else {
                    // Jump not taken
                    8 // 2 M-cycles
                }
            }

             // --- LD SP, d16 ---
             0x31 => {
                self.sp = self.fetch_word();
                12 // 3 M-cycles
            }

             // --- CB Prefix ---
             0xCB => {
                 let cb_opcode = self.fetch_byte();
                 self.execute_cb(cb_opcode) // Delegate to separate handler
             }


            // --- Catch Unimplemented Opcodes ---
            _ => {
                let current_pc = self.pc.wrapping_sub(1); // PC was already incremented
                panic!(
                    "Unimplemented opcode: 0x{:02X} at address 0x{:04X}",
                    opcode, current_pc
                );
            }
        }
    }

    /// Executes CB-prefixed opcodes.
    /// Returns the number of T-cycles the instruction took.
    fn execute_cb(&mut self, opcode: u8) -> u8 {
         // All CB instructions are at least 2 M-cycles (8 T-cycles) base
         let mut cycles = 8;
        match opcode {
            // --- BIT 7, H ---
            // Test bit 7 of register H
            0x7C => {
                let value = self.h;
                self.set_flag(FLAG_Z, (value & 0x80) == 0); // Set Z if bit 7 is 0
                self.set_flag(FLAG_N, false);              // Clear N
                self.set_flag(FLAG_H, true);               // Set H
                // C flag is not affected
            }

            // TODO: Implement all other CB-prefixed opcodes (RLC, RRC, RL, RR, SLA, SRA, SWAP, SRL, BIT, RES, SET)

            _ => {
                 let current_pc = self.pc.wrapping_sub(2); // PC incremented twice (CB + opcode)
                 panic!(
                    "Unimplemented CB opcode: 0x{:02X} at address 0x{:04X}",
                    opcode, current_pc
                 );
             }
        }
        cycles // Return T-cycles
    }

    // --- Register Getters/Setters (Combined) ---

    fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        // Lower 4 bits of F register are always zero
        self.f = (value & 0x00F0) as u8;
    }

    fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0x00FF) as u8;
    }

    fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0x00FF) as u8;
    }

    fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0x00FF) as u8;
    }

    // --- Flag Manipulation Helpers ---

    /// Sets or clears a specific flag bit in the F register.
    fn set_flag(&mut self, flag_mask: u8, set: bool) {
        if set {
            self.f |= flag_mask; // Set the bit
        } else {
            self.f &= !flag_mask; // Clear the bit
        }
    }

    /// Gets the state of a specific flag bit from the F register.
    fn get_flag(&self, flag_mask: u8) -> bool {
        (self.f & flag_mask) != 0
    }

    // --- Public accessors (Optional, for debugging/external interaction) ---
    pub fn pc(&self) -> u16 { self.pc }
    pub fn sp(&self) -> u16 { self.sp }
    pub fn registers(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8) { (self.a, self.f, self.b, self.c, self.d, self.e, self.h, self.l) }

}