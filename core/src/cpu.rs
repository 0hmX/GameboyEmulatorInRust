use crate::memory_bus::MemoryBus;

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

// Constants for Interrupt Handling
const VBLANK_VECTOR: u16 = 0x0040;
const LCD_STAT_VECTOR: u16 = 0x0048;
const TIMER_VECTOR: u16 = 0x0050;
const SERIAL_VECTOR: u16 = 0x0058;
const JOYPAD_VECTOR: u16 = 0x0060;

const IE_REGISTER: u16 = 0xFFFF; // Interrupt Enable Register address
const IF_REGISTER: u16 = 0xFF0F; // Interrupt Flag Register address

/// Represents the Game Boy CPU (Sharp LR35902).
pub struct Cpu<'a> {
    // Registers
    a: u8, // Accumulator
    f: u8, // Flags register (ZNHC----)
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
    stop_requested: bool, // Was STOP instruction executed?
    ime_scheduled: bool, // Should IME be enabled after the next instruction?

    total_cycles: u64 // For tracking total cycles executed (useful for timing)
}

impl<'a> Cpu<'a> {
    /// Creates a new CPU instance.
    /// `memory_bus`: A mutable reference to the MemoryBus instance.
    /// `skip_boot_rom`: If true, initializes registers to post-boot values and PC to 0x0100.
    ///                  If false, initializes for boot ROM execution (PC=0x0000).
    pub fn new(memory_bus: &'a mut MemoryBus, skip_boot_rom: bool) -> Self {
        // Values based on PanDocs - Post-Boot ROM values
        let (init_a, init_f, init_bc, init_de, init_hl, init_pc) = if skip_boot_rom {
            // Assuming DMG values after boot rom. CGB values differ slightly (A=0x11)
            // TODO: Differentiate between DMG/CGB initialization if needed
            (0x01, 0xB0, 0x0013, 0x00D8, 0x014D, 0x0100)
        } else {
            // Initial values before boot ROM runs (usually zeroed)
            (0x00, 0x00, 0x0000, 0x0000, 0x0000, 0x0000)
        };

        let mut cpu = Cpu {
            a: init_a,
            f: init_f,
            b: (init_bc >> 8) as u8,
            c: init_bc as u8,
            d: (init_de >> 8) as u8,
            e: init_de as u8,
            h: (init_hl >> 8) as u8,
            l: init_hl as u8,

            sp: 0xFFFE,  // Initial stack pointer is standard
            pc: init_pc,

            memory_bus,
            ime: false, // IME is disabled initially
            halted: false,
            stop_requested: false,
            ime_scheduled: false, // EI enables IME *after* the next instruction
            total_cycles: 0,
        };

        // If skipping boot ROM, we need to manually set some I/O registers
        // that the boot ROM would normally initialize.
        if skip_boot_rom {
             // Essential initial I/O register values (DMG)
             cpu.memory_bus.write_byte(0xFF05, 0x00); // TIMA
             cpu.memory_bus.write_byte(0xFF06, 0x00); // TMA
             cpu.memory_bus.write_byte(0xFF07, 0x00); // TAC
             cpu.memory_bus.write_byte(0xFF10, 0x80); // NR10
             cpu.memory_bus.write_byte(0xFF11, 0xBF); // NR11
             cpu.memory_bus.write_byte(0xFF12, 0xF3); // NR12
             cpu.memory_bus.write_byte(0xFF14, 0xBF); // NR14
             cpu.memory_bus.write_byte(0xFF16, 0x3F); // NR21
             cpu.memory_bus.write_byte(0xFF17, 0x00); // NR22
             cpu.memory_bus.write_byte(0xFF19, 0xBF); // NR24
             cpu.memory_bus.write_byte(0xFF1A, 0x7F); // NR30
             cpu.memory_bus.write_byte(0xFF1B, 0xFF); // NR31
             cpu.memory_bus.write_byte(0xFF1C, 0x9F); // NR32
             cpu.memory_bus.write_byte(0xFF1E, 0xBF); // NR33
             cpu.memory_bus.write_byte(0xFF20, 0xFF); // NR41
             cpu.memory_bus.write_byte(0xFF21, 0x00); // NR42
             cpu.memory_bus.write_byte(0xFF22, 0x00); // NR43
             cpu.memory_bus.write_byte(0xFF23, 0xBF); // NR44
             cpu.memory_bus.write_byte(0xFF24, 0x77); // NR50
             cpu.memory_bus.write_byte(0xFF25, 0xF3); // NR51
             cpu.memory_bus.write_byte(0xFF26, 0xF1); // NR52 - F1 for DMG, F0 for SGB
             cpu.memory_bus.write_byte(0xFF40, 0x91); // LCDC
             cpu.memory_bus.write_byte(0xFF41, 0x85); // STAT - Initial mode 1? Check PanDocs
             cpu.memory_bus.write_byte(0xFF42, 0x00); // SCY
             cpu.memory_bus.write_byte(0xFF43, 0x00); // SCX
             cpu.memory_bus.write_byte(0xFF45, 0x00); // LYC
             cpu.memory_bus.write_byte(0xFF47, 0xFC); // BGP
             cpu.memory_bus.write_byte(0xFF48, 0xFF); // OBP0
             cpu.memory_bus.write_byte(0xFF49, 0xFF); // OBP1
             cpu.memory_bus.write_byte(0xFF4A, 0x00); // WY
             cpu.memory_bus.write_byte(0xFF4B, 0x00); // WX
             cpu.memory_bus.write_byte(IE_REGISTER, 0x00);  // IE
             // IF (0xFF0F) starts at 0xE1 post-boot, indicating VBLANK occurred?
             // Boot ROM leaves it as 0xE1 (or similar like E0). Let's start clean for simplicity?
             cpu.memory_bus.write_byte(IF_REGISTER, 0x00); // IF
             // Write 0x01 to 0xFF50 to disable boot ROM mapping
             // The MemoryBus implementation should handle this if needed.
             cpu.memory_bus.write_byte(0xFF50, 0x01);
        }

        cpu
    }

    /// Executes a single CPU step: handles interrupts, then fetches and executes an instruction.
    /// Returns the number of **T-cycles** (clock cycles) consumed in this step.
    pub fn step(&mut self) -> u8 {
        // --- Handle pending EI instruction (enable IME after the instruction *following* EI) ---
        if self.ime_scheduled {
            self.ime = true;
            self.ime_scheduled = false;
        }

        // --- Check for and Handle Interrupts ---
        let interrupt_cycles = self.handle_interrupts();
        if interrupt_cycles > 0 {
            // Interrupt occurred, took precedence over HALT/instruction execution
            self.halted = false; // Wake up if halted
            self.stop_requested = false; // Wake up if stopped
            self.total_cycles += interrupt_cycles as u64;
            return interrupt_cycles;
        }

        // --- Handle HALT/STOP state ---
        if self.halted || self.stop_requested {
             // CPU is idle, consuming time. Does not fetch/execute.
             // Real hardware might enter low power in STOP.
             // TODO: Need logic for STOP mode exit (usually Joypad press)
             self.total_cycles += 4;
             return 4; // Consume 1 M-cycle (4 T-cycles)
        }

        // --- Fetch ---
        let opcode = self.fetch_byte();

        // --- Decode & Execute ---
        let instruction_cycles = self.execute(opcode);
        self.total_cycles += instruction_cycles as u64;

        instruction_cycles
    }

    /// Checks for pending and enabled interrupts and handles the highest priority one.
    /// Returns the number of cycles taken if an interrupt was handled (usually 20), or 0 otherwise.
    fn handle_interrupts(&mut self) -> u8 {
        if !self.ime && !self.halted { // Interrupts only handled if IME is on, *unless* HALTed
            return 0;
        }

        // Read Interrupt Flag (IF) and Interrupt Enable (IE) registers
        let if_flags = self.memory_bus.read_byte(IF_REGISTER);
        let ie_flags = self.memory_bus.read_byte(IE_REGISTER);

        // Check which interrupts are both requested (IF) and enabled (IE)
        let pending = if_flags & ie_flags;

        if pending == 0 {
            return 0; // No pending enabled interrupts
        }

        // --- Interrupt occurred ---

        // If CPU was HALTed, it wakes up now, even if IME is off.
        // However, the interrupt handler is only executed if IME *was* on.
        if self.halted {
             self.halted = false;
             if !self.ime {
                  return 0; // Wake from HALT but don't service interrupt if IME is off
             }
        }

        // If we get here, IME must be true. Disable it immediately.
        self.ime = false;
        self.ime_scheduled = false; // Cancel any pending EI

        // Determine highest priority interrupt (lower bit number = higher priority)
        let vector;
        let interrupt_bit;

        if pending & 0x01 != 0 { // VBlank (Priority 0)
            vector = VBLANK_VECTOR;
            interrupt_bit = 0;
        } else if pending & 0x02 != 0 { // LCD STAT (Priority 1)
            vector = LCD_STAT_VECTOR;
            interrupt_bit = 1;
        } else if pending & 0x04 != 0 { // Timer (Priority 2)
            vector = TIMER_VECTOR;
            interrupt_bit = 2;
        } else if pending & 0x08 != 0 { // Serial (Priority 3)
            vector = SERIAL_VECTOR;
            interrupt_bit = 3;
        } else if pending & 0x10 != 0 { // Joypad (Priority 4)
            vector = JOYPAD_VECTOR;
            interrupt_bit = 4;
        } else {
            // Should not happen if pending > 0
            return 0;
        }

        // Reset the corresponding bit in the IF register
        let current_if = self.memory_bus.read_byte(IF_REGISTER);
        self.memory_bus.write_byte(IF_REGISTER, current_if & !(1 << interrupt_bit));

        // Interrupt Handling Sequence takes 5 M-cycles (20 T-cycles):
        // 2 M-cycles delay/internal processing
        // 2 M-cycles pushing PC high byte, then low byte
        // 1 M-cycle jumping to the vector address
        self.push_word(self.pc);
        self.pc = vector;

        20 // Cycles consumed by handling the interrupt
    }


    /// Fetches the next byte from memory at the PC and increments the PC.
    /// This uses the integrated MemoryBus.
    fn fetch_byte(&mut self) -> u8 {
        let byte = self.memory_bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    /// Fetches the next word (16 bits, little-endian) from memory at the PC
    /// and increments the PC by 2. Uses the MemoryBus.
    fn fetch_word(&mut self) -> u16 {
        let low = self.fetch_byte() as u16;
        let high = self.fetch_byte() as u16;
        (high << 8) | low
    }

    /// Pushes a 16-bit value onto the stack. Uses the MemoryBus.
    fn push_word(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(1);
        self.memory_bus.write_byte(self.sp, (value >> 8) as u8); // High byte
        self.sp = self.sp.wrapping_sub(1);
        self.memory_bus.write_byte(self.sp, (value & 0xFF) as u8); // Low byte
    }

    /// Pops a 16-bit value from the stack. Uses the MemoryBus.
    fn pop_word(&mut self) -> u16 {
        let low = self.memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = self.memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (high << 8) | low
    }


    fn jp_cc(&mut self, condition: bool) -> u8 {
        let jump_addr = self.fetch_word(); // Always fetches the address
        if condition {
            self.pc = jump_addr;
            16 // 4 M-cycles (fetch + execute jump)
        } else {
            12 // 3 M-cycles (fetch only)
        }
    }
    fn daa(&mut self) {
        let mut adjustment = 0u8;
        let mut set_carry = false;

        // Conditions for adjustment (based on PanDocs/Z80 behavior)
        let n_flag = self.get_flag(FLAG_N); // Subtract flag (true if last op was subtraction)
        let h_flag = self.get_flag(FLAG_H); // Half Carry flag
        let c_flag = self.get_flag(FLAG_C); // Carry flag

        if !n_flag { // After addition
            // If lower nibble > 9 or Half Carry was set, add 0x06
            if h_flag || (self.a & 0x0F) > 9 {
                adjustment |= 0x06;
            }
            // If upper nibble > 9 or Carry was set, add 0x60
            if c_flag || self.a > 0x99 {
                adjustment |= 0x60;
                set_carry = true; // Set Carry flag if adjustment needed for upper nibble
            }
            self.a = self.a.wrapping_add(adjustment);
        } else { // After subtraction
            // If Half Carry was set, subtract 0x06 (or add 0xFA effectively)
            if h_flag {
                 adjustment |= 0x06;
                 // No upper nibble adjustment based on H flag during subtraction? Check manuals.
                 // Seems like Z80 doesn't add 0x60 here. GB Z80 might differ slightly but likely follows.
            }
             // If Carry was set, subtract 0x60 (or add 0xA0 effectively)
             if c_flag {
                 adjustment |= 0x60;
                 set_carry = true; // Carry remains set if it was set before
             }
             self.a = self.a.wrapping_sub(adjustment);
        }

        // Set flags:
        self.set_flag(FLAG_Z, self.a == 0);
        // N flag is not affected by DAA
        self.set_flag(FLAG_H, false); // H flag is always reset by DAA
        self.set_flag(FLAG_C, set_carry || c_flag); // Set C if adjustment caused carry OR if it was already set (for subtraction)
    }
    fn ret_cc(&mut self, condition: bool) -> u8 {
        if condition {
            // Return taken: Pop PC + execution time
            self.pc = self.pop_word();
            20 // 5 M-cycles (read condition + pop + jump)
        } else {
            // Return not taken: Only read condition
            8 // 2 M-cycles
        }
    }
    fn call_cc(&mut self, condition: bool) -> u8 {
        let jump_addr = self.fetch_word(); // Always fetches the address (PC now points after nn)
        if condition {
            // Call taken: Push PC + jump
            self.push_word(self.pc); // Push the address *after* the CALL instruction
            self.pc = jump_addr;
            24 // 6 M-cycles (read condition + fetch addr + push + jump)
        } else {
            // Call not taken: Only read condition + fetch address
            12 // 3 M-cycles
        }
    }
    fn rst(&mut self, vector_offset: u16) -> u8 {
        self.push_word(self.pc); // Push address *after* RST instruction
        self.pc = vector_offset;
        16 // 4 M-cycles
    }
    fn add_sp_i8(&mut self) {
        let offset = self.fetch_byte() as i8; // Signed offset
        let value = offset as i16 as u16; // Convert to u16 correctly handling sign extension
        let sp = self.sp;

        // Calculate result
        let result = sp.wrapping_add(value);

        // Calculate flags based on lower byte addition (like ADD HL, rr)
        // Carry: Check carry out of bit 7 when adding lower bytes
        let carry = (sp & 0x00FF) + (value & 0x00FF) > 0x00FF;
        // Half Carry: Check carry out of bit 3 when adding lower bytes
        let half_carry = (sp & 0x000F) + (value & 0x000F) > 0x000F;

        self.sp = result;

        // Set flags for ADD SP, i8
        self.set_flag(FLAG_Z | FLAG_N, false); // Z and N are always reset
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
    }
    fn ld_hl_sp_i8(&mut self) {
        let offset = self.fetch_byte() as i8; // Signed offset
        let value = offset as i16 as u16; // Convert to u16 correctly handling sign extension
        let sp = self.sp;

        // Calculate result
        let result = sp.wrapping_add(value);

        // Calculate flags based on lower byte addition (same as ADD SP, i8)
        let carry = (sp & 0x00FF) + (value & 0x00FF) > 0x00FF;
        let half_carry = (sp & 0x000F) + (value & 0x000F) > 0x000F;

        self.set_hl(result);

        // Set flags for LD HL, SP+i8
        self.set_flag(FLAG_Z | FLAG_N, false); // Z and N are always reset
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
    }

    /// Decodes and executes a fetched opcode.
    /// Returns the number of T-cycles the instruction took.
    /// Instructions that read/write memory implicitly use `self.memory_bus`.
    fn execute(&mut self, opcode: u8) -> u8 {
        // --- Simple example instructions (already compatible) ---
        match opcode {
            // NOP
            0x00 => 4,

            // LD BC, d16
            0x01 => { let value = self.fetch_word(); self.set_bc(value); 12 }
            // LD (BC), A
            0x02 => { self.memory_bus.write_byte(self.get_bc(), self.a); 8 },
            // INC BC
            0x03 => { self.set_bc(self.get_bc().wrapping_add(1)); 8 },
            // INC B
            0x04 => { self.b = self.inc_u8(self.b); 4 },
            // DEC B
            0x05 => { self.b = self.dec_u8(self.b); 4 },
             // LD B, d8
             0x06 => { self.b = self.fetch_byte(); 8 },
            // RLCA (Rotate Left A through Carry)
            0x07 => { self.a = self.rlc(self.a); self.set_flag(FLAG_Z, false); 4 }, // RLCA clears Z flag

            // LD (a16), SP
            0x08 => {
                 let addr = self.fetch_word();
                 self.memory_bus.write_byte(addr, (self.sp & 0xFF) as u8);
                 self.memory_bus.write_byte(addr.wrapping_add(1), (self.sp >> 8) as u8);
                 20
             },
            // ADD HL, BC
            0x09 => { let val = self.get_bc(); self.add_hl(val); 8 },
             // LD A, (BC)
             0x0A => { self.a = self.memory_bus.read_byte(self.get_bc()); 8 },
             // DEC BC
             0x0B => { self.set_bc(self.get_bc().wrapping_sub(1)); 8 },
             // INC C
             0x0C => { self.c = self.inc_u8(self.c); 4 },
             // DEC C
             0x0D => { self.c = self.dec_u8(self.c); 4 },
             // LD C, d8
             0x0E => { self.c = self.fetch_byte(); 8 },
            // RRCA (Rotate Right A through Carry)
            0x0F => { self.a = self.rrc(self.a); self.set_flag(FLAG_Z, false); 4 }, // RRCA clears Z flag


            // STOP (Needs special handling in main loop for potential 0x00 byte following)
            0x10 => {
                 // Fetch the potential 0x00 byte if present
                 // let next_byte = self.memory_bus.read_byte(self.pc);
                 // if next_byte == 0x00 { self.pc = self.pc.wrapping_add(1); } // Consume 0x00
                 // TODO: Handle CGB speed switching if applicable.
                 self.stop_requested = true;
                 4
             },
            // LD DE, d16
            0x11 => { 
                let word = self.fetch_word();
                self.set_de(word); 
            12 },
             // LD (DE), A
             0x12 => { self.memory_bus.write_byte(self.get_de(), self.a); 8 },
             // INC DE
             0x13 => { self.set_de(self.get_de().wrapping_add(1)); 8 },
            // INC D
            0x14 => { self.d = self.inc_u8(self.d); 4 },
            // DEC D
            0x15 => { self.d = self.dec_u8(self.d); 4 },
             // LD D, d8
             0x16 => { self.d = self.fetch_byte(); 8 },
            // RLA (Rotate Left A)
            0x17 => { self.a = self.rl(self.a); self.set_flag(FLAG_Z, false); 4 }, // RLA clears Z flag

            // JR r8 (Unconditional Relative Jump)
            0x18 => self.jr_cc(true), // Condition always true for JR
            // ADD HL, DE
            0x19 => { let val = self.get_de(); self.add_hl(val); 8 },
             // LD A, (DE)
             0x1A => { self.a = self.memory_bus.read_byte(self.get_de()); 8 },
             // DEC DE
             0x1B => { self.set_de(self.get_de().wrapping_sub(1)); 8 },
            // INC E
            0x1C => { self.e = self.inc_u8(self.e); 4 },
            // DEC E
            0x1D => { self.e = self.dec_u8(self.e); 4 },
            // LD E, d8
            0x1E => { self.e = self.fetch_byte(); 8 },
            // RRA (Rotate Right A)
            0x1F => { self.a = self.rr(self.a); self.set_flag(FLAG_Z, false); 4 }, // RRA clears Z flag


            // JR NZ, r8
            0x20 => self.jr_cc(!self.get_flag(FLAG_Z)),
            // LD HL, d16
            0x21 => { let val = self.fetch_word();
                self.set_hl(val);
                 12 },
            // LD (HL+), A
            0x22 => {
                 let addr = self.get_hl();
                 self.memory_bus.write_byte(addr, self.a);
                 self.set_hl(addr.wrapping_add(1));
                 8
            },
            // INC HL
            0x23 => { self.set_hl(self.get_hl().wrapping_add(1)); 8 },
             // INC H
             0x24 => { self.h = self.inc_u8(self.h); 4 },
             // DEC H
             0x25 => { self.h = self.dec_u8(self.h); 4 },
             // LD H, d8
             0x26 => { self.h = self.fetch_byte(); 8 },
            // DAA (Decimal Adjust Accumulator)
            0x27 => { self.daa(); 4 },

            // JR Z, r8
            0x28 => self.jr_cc(self.get_flag(FLAG_Z)),
            // ADD HL, HL
            0x29 => { let val = self.get_hl(); self.add_hl(val); 8 },
            // LD A, (HL+)
            0x2A => {
                 let addr = self.get_hl();
                 self.a = self.memory_bus.read_byte(addr);
                 self.set_hl(addr.wrapping_add(1));
                 8
             },
            // DEC HL
            0x2B => { self.set_hl(self.get_hl().wrapping_sub(1)); 8 },
             // INC L
             0x2C => { self.l = self.inc_u8(self.l); 4 },
             // DEC L
             0x2D => { self.l = self.dec_u8(self.l); 4 },
             // LD L, d8
             0x2E => { self.l = self.fetch_byte(); 8 },
            // CPL (Complement A)
            0x2F => {
                self.a = !self.a;
                self.set_flag(FLAG_N | FLAG_H, true); // Set N and H flags
                4
            },

            // JR NC, r8
            0x30 => self.jr_cc(!self.get_flag(FLAG_C)),
            // LD SP, d16
            0x31 => { self.sp = self.fetch_word(); 12 },
             // LD (HL-), A
             0x32 => {
                 let addr = self.get_hl();
                 self.memory_bus.write_byte(addr, self.a);
                 self.set_hl(addr.wrapping_sub(1));
                 8
            },
             // INC SP
             0x33 => { self.sp = self.sp.wrapping_add(1); 8 },
            // INC (HL)
            0x34 => {
                let addr = self.get_hl();
                let value = self.memory_bus.read_byte(addr);
                let result = self.inc_u8(value);
                self.memory_bus.write_byte(addr, result);
                12
            },
            // DEC (HL)
            0x35 => {
                let addr = self.get_hl();
                let value = self.memory_bus.read_byte(addr);
                let result = self.dec_u8(value);
                self.memory_bus.write_byte(addr, result);
                12
            },
            // LD (HL), d8
            0x36 => {
                let value = self.fetch_byte();
                self.memory_bus.write_byte(self.get_hl(), value);
                12
            },
            // SCF (Set Carry Flag)
            0x37 => {
                self.set_flag(FLAG_N | FLAG_H, false); // Clear N and H
                self.set_flag(FLAG_C, true);           // Set C
                4
            },

            // JR C, r8
            0x38 => self.jr_cc(self.get_flag(FLAG_C)),
            // ADD HL, SP
            0x39 => { let val = self.sp; self.add_hl(val); 8 },
            // LD A, (HL-)
            0x3A => {
                 let addr = self.get_hl();
                 self.a = self.memory_bus.read_byte(addr);
                 self.set_hl(addr.wrapping_sub(1));
                 8
             },
             // DEC SP
             0x3B => { self.sp = self.sp.wrapping_sub(1); 8 },
            // INC A
            0x3C => { self.a = self.inc_u8(self.a); 4 },
             // DEC A
             0x3D => { self.a = self.dec_u8(self.a); 4 },
            // LD A, d8
            0x3E => { self.a = self.fetch_byte(); 8 },
            // CCF (Complement Carry Flag)
            0x3F => {
                let current_c = self.get_flag(FLAG_C);
                self.set_flag(FLAG_N | FLAG_H, false); // Clear N and H
                self.set_flag(FLAG_C, !current_c);     // Flip C
                4
            },

             // LD B, B to LD L, L (effectively NOPs)
             // LD r8, r8'
             0x40..=0x7F => {
                 // Exclude HALT (0x76) which is handled below
                 if opcode == 0x76 { self.halt(); 4 } else { self.ld_r8_r8(opcode) }
                 // This range includes 0x78 (LD A, B)
             },
             // LD (HL), r8 where r8 = B,C,D,E,H,L,A
             0x70..=0x75 | 0x77 => {
                 let val = match opcode {
                    0x70 => self.b, 0x71 => self.c, 0x72 => self.d,
                    0x73 => self.e, 0x74 => self.h, 0x75 => self.l,
                    0x77 => self.a,
                    _ => unreachable!(), // Should not happen
                 };
                 self.memory_bus.write_byte(self.get_hl(), val);
                 8
             },
             // HALT
             0x76 => { self.halt(); 4 },

            // LD r8, (HL) where r8 = B,C,D,E,H,L,A
            0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x7E => {
                let val = self.memory_bus.read_byte(self.get_hl());
                 match opcode {
                     0x46 => self.b = val, 0x4E => self.c = val,
                     0x56 => self.d = val, 0x5E => self.e = val,
                     0x66 => self.h = val, 0x6E => self.l = val,
                     0x7E => self.a = val,
                     _ => unreachable!(),
                 };
                8
            },

            // --- ADD A, r8 / ADC A, r8 / SUB A, r8 / SBC A, r8 ---
             // Includes (HL) operand which takes 8 cycles
            0x80..=0x9F => {
                let cycles = if (opcode & 0x07) == 0x06 { 8 } else { 4 };
                self.alu_op(opcode);
                cycles
            },

            // --- AND A, r8 / XOR A, r8 / OR A, r8 / CP A, r8 ---
            // Includes (HL) operand which takes 8 cycles
            // This range includes 0xB1 (OR C) and 0xAF (XOR A)
            0xA0..=0xBF => {
                let cycles = if (opcode & 0x07) == 0x06 { 8 } else { 4 };
                self.alu_op(opcode);
                cycles
            },

            // --- Conditional Returns ---
            // RET NZ
            0xC0 => self.ret_cc(!self.get_flag(FLAG_Z)),
            // POP BC
            0xC1 => { let val = self.pop_word(); self.set_bc(val); 12 },
            // JP NZ, nn
            0xC2 => self.jp_cc(!self.get_flag(FLAG_Z)),
            // JP nn
            0xC3 => { self.pc = self.fetch_word(); 16 },
            // CALL NZ, nn
            0xC4 => self.call_cc(!self.get_flag(FLAG_Z)),
            // PUSH BC
            0xC5 => { self.push_word(self.get_bc()); 16 },
            // ADD A, n8
            0xC6 => { let v = self.fetch_byte(); self.add_a(v, false); 8 },
            // RST 00H
            0xC7 => { self.rst(0x00); 16 },

            // RET Z
            0xC8 => self.ret_cc(self.get_flag(FLAG_Z)),
            // RET (Unconditional Return)
            0xC9 => { self.pc = self.pop_word(); 16 },
            // JP Z, nn
            0xCA => self.jp_cc(self.get_flag(FLAG_Z)),
            // CB Prefix
            0xCB => { let val = self.fetch_byte();
                 self.execute_cb(val)},
            // CALL Z, nn
            0xCC => self.call_cc(self.get_flag(FLAG_Z)),
            // CALL nn
            0xCD => {
                let addr = self.fetch_word();
                self.push_word(self.pc); // Push address *after* CALL instruction
                self.pc = addr;
                24 // 6 M-cycles
            },
            // ADC A, n8
            0xCE => { let v = self.fetch_byte(); self.add_a(v, true); 8 },
            // RST 08H
            0xCF => { self.rst(0x08); 16 },


            // RET NC
            0xD0 => self.ret_cc(!self.get_flag(FLAG_C)),
            // POP DE
            0xD1 => { let val = self.pop_word(); self.set_de(val); 12 },
            // JP NC, nn
            0xD2 => self.jp_cc(!self.get_flag(FLAG_C)),
            // Opcode 0xD3 is invalid/unused
            0xD3 => { panic!("Invalid opcode: 0xD3 at {:04X}", self.pc.wrapping_sub(1)); }
            // CALL NC, nn
            0xD4 => self.call_cc(!self.get_flag(FLAG_C)),
            // PUSH DE
            0xD5 => { self.push_word(self.get_de()); 16 },
            // SUB A, n8
            0xD6 => { let v = self.fetch_byte(); self.sub_a(v, false); 8 },
            // RST 10H
            0xD7 => { self.rst(0x10); 16 },

            // RET C
            0xD8 => self.ret_cc(self.get_flag(FLAG_C)),
            // RETI (Return from Interrupt)
            0xD9 => {
                self.pc = self.pop_word();
                self.ime = true; // Enable interrupts after RETI
                16
            },
            // JP C, nn
            0xDA => self.jp_cc(self.get_flag(FLAG_C)),
             // Opcode 0xDB is invalid/unused
             0xDB => { panic!("Invalid opcode: 0xDB at {:04X}", self.pc.wrapping_sub(1)); }
            // CALL C, nn
            0xDC => self.call_cc(self.get_flag(FLAG_C)),
            // Opcode 0xDD is invalid/unused
             0xDD => { panic!("Invalid opcode: 0xDD at {:04X}", self.pc.wrapping_sub(1)); }
            // SBC A, n8
            0xDE => { let v = self.fetch_byte(); self.sub_a(v, true); 8 },
            // RST 18H
            0xDF => { self.rst(0x18); 16 },


            // LDH (a8), A --- Write A to 0xFF00 + n8
            0xE0 => {
                let offset = self.fetch_byte() as u16;
                self.memory_bus.write_byte(0xFF00 + offset, self.a);
                12
            },
            // POP HL
            0xE1 => { let val = self.pop_word(); self.set_hl(val); 12 },
             // LD (C), A --- Write A to 0xFF00 + C
             0xE2 => {
                self.memory_bus.write_byte(0xFF00 + self.c as u16, self.a);
                8
            },
            // Opcodes 0xE3, 0xE4 are invalid/unused
            0xE3 | 0xE4 => { panic!("Invalid opcode: {:02X} at {:04X}", opcode, self.pc.wrapping_sub(1)); }
            // PUSH HL
            0xE5 => { self.push_word(self.get_hl()); 16 },
            // AND A, n8
            0xE6 => { let v = self.fetch_byte(); self.and_a(v); 8 },
            // RST 20H
            0xE7 => { self.rst(0x20); 16 },

            // ADD SP, r8 (signed immediate)
            0xE8 => { self.add_sp_i8(); 16 },
            // JP HL (Jump to address in HL)
            0xE9 => { self.pc = self.get_hl(); 4 },
            // LD (a16), A
            0xEA => {
                let addr = self.fetch_word();
                self.memory_bus.write_byte(addr, self.a);
                16
            },
            // Opcodes 0xEB, 0xEC, 0xED are invalid/unused
            0xEB | 0xEC | 0xED => { panic!("Invalid opcode: {:02X} at {:04X}", opcode, self.pc.wrapping_sub(1)); }
            // XOR A, n8
            0xEE => { let v = self.fetch_byte(); self.xor_a(v); 8 },
            // RST 28H
            0xEF => { self.rst(0x28); 16 },


            // LDH A, (a8) --- Read from 0xFF00 + n8 into A
            0xF0 => {
                let offset = self.fetch_byte() as u16;
                self.a = self.memory_bus.read_byte(0xFF00 + offset);
                12
            },
            // POP AF
            0xF1 => { let val = self.pop_word(); self.set_af(val); 12 },
            // LD A, (C) --- Read from 0xFF00 + C into A
            0xF2 => {
                 self.a = self.memory_bus.read_byte(0xFF00 + self.c as u16);
                 8
             },
            // DI --- Disable Interrupts
            0xF3 => {
                self.ime = false;
                self.ime_scheduled = false; // Cancel pending EI
                4
            },
             // Opcode 0xF4 is invalid/unused
             0xF4 => { panic!("Invalid opcode: 0xF4 at {:04X}", self.pc.wrapping_sub(1)); }
            // PUSH AF
            0xF5 => { self.push_word(self.get_af()); 16 },
            // OR A, n8
            0xF6 => { let v = self.fetch_byte(); self.or_a(v); 8 },
            // RST 30H
            0xF7 => { self.rst(0x30); 16 },

            // LD HL, SP+r8 (signed immediate)
            0xF8 => { self.ld_hl_sp_i8(); 12 },
            // LD SP, HL
            0xF9 => { self.sp = self.get_hl(); 8 },
            // LD A, (a16)
            0xFA => {
                 let addr = self.fetch_word();
                 self.a = self.memory_bus.read_byte(addr);
                 16
            },
            // EI --- Enable Interrupts (delayed)
            0xFB => {
                // IME is enabled *after* the instruction following EI
                self.ime_scheduled = true;
                4
            },
            // Opcodes 0xFC, 0xFD are invalid/unused
            0xFC | 0xFD => { panic!("Invalid opcode: {:02X} at {:04X}", opcode, self.pc.wrapping_sub(1)); }
            // CP A, n8
            0xFE => {
                 let value = self.fetch_byte();
                 self.cp_a(value);
                 8
            },
            // RST 38H
            0xFF => { self.rst(0x38); 16 },

            // This catch-all should ideally not be reached if all 256 opcodes are handled
             _ => {
                let current_pc = self.pc.wrapping_sub(1); // PC was already incremented by fetch
                panic!(
                    "Reached end of match - Unhandled opcode: 0x{:02X} at address 0x{:04X}\nCPU State: AF={:04X} BC={:04X} DE={:04X} HL={:04X} SP={:04X} IME={}",
                    opcode, current_pc, self.get_af(), self.get_bc(), self.get_de(), self.get_hl(), self.sp, self.ime
                );
             }
        }
    }

    /// Executes CB-prefixed opcodes.
    /// Returns the number of T-cycles the instruction took.
    /// Operations involving (HL) take more cycles.
    fn execute_cb(&mut self, opcode: u8) -> u8 {
         // Most CB ops are 8 cycles, (HL) ops are 16 (except BIT which is 12)
         let mut cycles = 8;
         let target_reg_code = opcode & 0x07; // 0-5: B,C,D,E,H,L, 6: (HL), 7: A
         let operation_code = opcode >> 3; // Identifies the operation type (RLC, BIT, SET, etc.)

         let value = match target_reg_code {
             0 => self.b, 1 => self.c, 2 => self.d, 3 => self.e,
             4 => self.h, 5 => self.l,
             6 => {
                 cycles = if operation_code >= 8 && operation_code < 16 { 12 } else { 16 }; // BIT (HL) is 12, others 16
                 self.memory_bus.read_byte(self.get_hl())
             },
             7 => self.a,
             _ => unreachable!(), // Should not happen with & 0x07
         };

         let bit_index = (opcode >> 3) & 0x07; // For BIT, RES, SET (0-7)

         let result = match operation_code {
             // --- Rotates/Shifts ---
             0 => self.rlc(value), // RLC r8 / (HL)
             1 => self.rrc(value), // RRC r8 / (HL)
             2 => self.rl(value),  // RL r8 / (HL)
             3 => self.rr(value),  // RR r8 / (HL)
             4 => self.sla(value), // SLA r8 / (HL)
             5 => self.sra(value), // SRA r8 / (HL)
             6 => self.swap(value),// SWAP r8 / (HL)
             7 => self.srl(value), // SRL r8 / (HL)

             // --- BIT, RES, SET --- (bit_index matters here)
             8..=15 => { // BIT b, r8 / (HL)
                 self.op_bit(bit_index, value);
                 value // BIT doesn't change the value, only flags
             }
             16..=23 => { // RES b, r8 / (HL)
                 value & !(1 << bit_index)
             }
              24..=31 => { // SET b, r8 / (HL)
                 value | (1 << bit_index)
             }
            _ => unreachable!(),
         };

        // Write result back if it wasn't a BIT operation
        if operation_code < 8 || operation_code >= 16 {
             match target_reg_code {
                 0 => self.b = result, 1 => self.c = result, 2 => self.d = result, 3 => self.e = result,
                 4 => self.h = result, 5 => self.l = result,
                 6 => self.memory_bus.write_byte(self.get_hl(), result),
                 7 => self.a = result,
                 _ => unreachable!(),
             };
        }

        cycles
    }

    // --- Register Getters/Setters (Combined) ---
    // (Unchanged from your code - they are correct)
    fn get_af(&self) -> u16 { ((self.a as u16) << 8) | (self.f as u16) }
    fn set_af(&mut self, value: u16) { self.a = (value >> 8) as u8; self.f = (value & 0x00F0) as u8; } // Mask low bits
    fn get_bc(&self) -> u16 { ((self.b as u16) << 8) | (self.c as u16) }
    fn set_bc(&mut self, value: u16) { self.b = (value >> 8) as u8; self.c = (value & 0x00FF) as u8; }
    fn get_de(&self) -> u16 { ((self.d as u16) << 8) | (self.e as u16) }
    fn set_de(&mut self, value: u16) { self.d = (value >> 8) as u8; self.e = (value & 0x00FF) as u8; }
    fn get_hl(&self) -> u16 { ((self.h as u16) << 8) | (self.l as u16) }
    fn set_hl(&mut self, value: u16) { self.h = (value >> 8) as u8; self.l = (value & 0x00FF) as u8; }

    // --- Flag Manipulation Helpers ---
    // (Unchanged - correct)
    fn set_flag(&mut self, flag_mask: u8, set: bool) { if set { self.f |= flag_mask; } else { self.f &= !flag_mask; } }
    fn get_flag(&self, flag_mask: u8) -> bool { (self.f & flag_mask) != 0 }

     // --- CPU State Control ---
     fn halt(&mut self) {
        // HALT bug: If IME=0 and (IE & IF) != 0, HALT fails to stop execution,
        // and the instruction *after* HALT is executed twice (PC doesn't increment).
        // Simple check here, proper handling requires loop structure adjustment.
        let ie = self.memory_bus.read_byte(IE_REGISTER);
        let iflags = self.memory_bus.read_byte(IF_REGISTER);
        if !self.ime && (ie & iflags) != 0 {
            // HALT bug triggered - don't actually halt.
            // The main loop should handle the no-PC-increment issue.
            // For now, we just don't set self.halted = true;
            println!("WARN: HALT bug triggered? IME=0, IE&IF={:02X}", ie & iflags);
        } else {
            self.halted = true;
        }
    }

    // --- Instruction Helpers (Examples - Need to implement all ALU/CB ops) ---

    // JR cc, r8
    fn jr_cc(&mut self, condition: bool) -> u8 {
        let relative_offset = self.fetch_byte() as i8; // Read signed offset
        if condition {
            // Jump taken
            let current_pc = self.pc;
            self.pc = current_pc.wrapping_add(relative_offset as i16 as u16); // Signed addition
            12 // 3 M-cycles
        } else {
            // Jump not taken
            8 // 2 M-cycles
        }
    }

    // INC r8
    fn inc_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        // Half carry: check if bit 3 overflows into bit 4
        self.set_flag(FLAG_H, (value & 0x0F) + 1 > 0x0F);
        // C flag is not affected by INC
        result
    }

    // DEC r8
    fn dec_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        // Half borrow: check if lower nibble borrowed from bit 4 (0x0F -> 0x0E is fine, 0x00 -> 0xFF borrows)
        self.set_flag(FLAG_H, (value & 0x0F) == 0);
        // C flag is not affected by DEC
        result
    }

     // ADD HL, rr
     fn add_hl(&mut self, value: u16) {
        let hl = self.get_hl();
        let (result, carry) = hl.overflowing_add(value);
        // Half carry check: Add lower bytes, check carry from bit 11 to 12
        let half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;

        self.set_hl(result);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
     }


    // LD r8, r8' (Generic handler for 0x40-0x7F range, excluding HALT)
    fn ld_r8_r8(&mut self, opcode: u8) -> u8 {
        let source_reg_code = opcode & 0x07; // B=0, C=1, D=2, E=3, H=4, L=5, (HL)=6, A=7
        let dest_reg_code = (opcode >> 3) & 0x07; // Same coding

        // Reading from (HL) takes longer
        let read_cycles = if source_reg_code == 6 { 8 } else { 4 };
        // Writing to (HL) takes longer
        let write_cycles = if dest_reg_code == 6 { 8 } else { 4 };

        // If source and dest are both (HL), it still only takes 8 cycles total? Check manuals.
        // Let's assume simple case: base 4 + extra 4 if source is (HL) + extra 4 if dest is (HL) is wrong.
        // It's just 4 cycles for reg-reg, 8 cycles for reg-(HL) or (HL)-reg.

         let value = match source_reg_code {
             0 => self.b, 1 => self.c, 2 => self.d, 3 => self.e,
             4 => self.h, 5 => self.l, 6 => self.memory_bus.read_byte(self.get_hl()), 7 => self.a,
             _ => unreachable!(),
         };

         match dest_reg_code {
             0 => self.b = value, 1 => self.c = value, 2 => self.d = value, 3 => self.e = value,
             4 => self.h = value, 5 => self.l = value,
             6 => self.memory_bus.write_byte(self.get_hl(), value),
             7 => self.a = value,
             _ => unreachable!(),
         };

         if source_reg_code == 6 || dest_reg_code == 6 { 8 } else { 4 }
    }

    // Basic ALU operation handler (Needs more work for flags and (HL))
    fn alu_op(&mut self, opcode: u8) {
        let operation = (opcode >> 3) & 0x07; // 0:ADD, 1:ADC, 2:SUB, 3:SBC, 4:AND, 5:XOR, 6:OR, 7:CP
        let operand_code = opcode & 0x07; // 0:B..7:A, 6=(HL)

        // TODO: Handle read from (HL) taking longer
        let operand = match operand_code {
            0 => self.b, 1 => self.c, 2 => self.d, 3 => self.e,
            4 => self.h, 5 => self.l, 6 => self.memory_bus.read_byte(self.get_hl()), 7 => self.a,
             _ => unreachable!(),
        };

        match operation {
            0 => self.add_a(operand, false), // ADD
            1 => self.add_a(operand, true),  // ADC
            2 => self.sub_a(operand, false), // SUB
            3 => self.sub_a(operand, true),  // SBC
            4 => self.and_a(operand),        // AND
            5 => self.xor_a(operand),        // XOR
            6 => self.or_a(operand),         // OR
            7 => self.cp_a(operand),         // CP
             _ => unreachable!(),
        }
    }

     // --- Actual ALU operations ---
     fn add_a(&mut self, value: u8, use_carry: bool) {
        let carry_in = if use_carry && self.get_flag(FLAG_C) { 1 } else { 0 };
        let (res1, c1) = self.a.overflowing_add(value);
        let (result, c2) = res1.overflowing_add(carry_in);
        let carry_out = c1 || c2;

        // Half carry: Check carry from bit 3 to bit 4
        let half_carry = (self.a & 0x0F) + (value & 0x0F) + carry_in > 0x0F;

        self.a = result;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry_out);
    }

     fn sub_a(&mut self, value: u8, use_carry: bool) {
        let carry_in = if use_carry && self.get_flag(FLAG_C) { 1 } else { 0 }; // Carry acts as borrow here
        let (res1, b1) = self.a.overflowing_sub(value);
        let (result, b2) = res1.overflowing_sub(carry_in);
        let borrow_out = b1 || b2;

        // Half borrow: Check borrow from bit 4 for bit 3
        let half_borrow = (self.a & 0x0F) < (value & 0x0F) + carry_in;

        self.a = result;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, half_borrow);
        self.set_flag(FLAG_C, borrow_out); // Borrow becomes carry flag
    }

    fn and_a(&mut self, value: u8) {
        self.a &= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true); // AND sets H flag
        self.set_flag(FLAG_C, false);
    }

    fn xor_a(&mut self, value: u8) {
        self.a ^= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false); // XOR clears N, H, C
    }

    fn or_a(&mut self, value: u8) {
        self.a |= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false); // OR clears N, H, C
    }

     fn cp_a(&mut self, value: u8) {
         // CP performs a subtraction but discards the result, only setting flags
        let temp_a = self.a; // Keep original A
        self.sub_a(value, false); // Perform SUB logic
        self.a = temp_a; // Restore A
     }


      // --- CB Prefix Operations ---
    fn rlc(&mut self, value: u8) -> u8 {
        let carry = (value >> 7) & 1;
        let result = value.rotate_left(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }

    fn rrc(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let result = value.rotate_right(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }

    fn rl(&mut self, value: u8) -> u8 {
        let old_carry = self.get_flag(FLAG_C) as u8;
        let new_carry = (value >> 7) & 1;
        let result = (value << 1) | old_carry;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, new_carry != 0);
        result
    }

    fn rr(&mut self, value: u8) -> u8 {
        let old_carry = self.get_flag(FLAG_C) as u8;
        let new_carry = value & 1;
        let result = (value >> 1) | (old_carry << 7);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, new_carry != 0);
        result
    }

     fn sla(&mut self, value: u8) -> u8 {
         let carry = (value >> 7) & 1;
         let result = value << 1;
         self.set_flag(FLAG_Z, result == 0);
         self.set_flag(FLAG_N | FLAG_H, false);
         self.set_flag(FLAG_C, carry != 0);
         result
     }

     fn sra(&mut self, value: u8) -> u8 {
         let carry = value & 1;
         let result = (value >> 1) | (value & 0x80); // Keep MSB
         self.set_flag(FLAG_Z, result == 0);
         self.set_flag(FLAG_N | FLAG_H, false);
         self.set_flag(FLAG_C, carry != 0);
         result
     }

     fn swap(&mut self, value: u8) -> u8 {
         let result = value.rotate_right(4); // Swaps upper and lower nibbles
         self.set_flag(FLAG_Z, result == 0);
         self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
         result
     }

     fn srl(&mut self, value: u8) -> u8 {
         let carry = value & 1;
         let result = value >> 1;
         self.set_flag(FLAG_Z, result == 0);
         self.set_flag(FLAG_N | FLAG_H, false);
         self.set_flag(FLAG_C, carry != 0);
         result
     }

      fn op_bit(&mut self, bit: u8, value: u8) {
         let result_zero = (value >> bit) & 1 == 0;
         self.set_flag(FLAG_Z, result_zero);
         self.set_flag(FLAG_N, false);
         self.set_flag(FLAG_H, true);
         // C flag is not affected by BIT
     }

    // --- Public accessors (Optional, for debugging/external interaction) ---
    pub fn pc(&self) -> u16 { self.pc }
    pub fn sp(&self) -> u16 { self.sp }
    pub fn registers(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8) { (self.a, self.f, self.b, self.c, self.d, self.e, self.h, self.l) }
    pub fn ime(&self) -> bool { self.ime }
    pub fn halted(&self) -> bool { self.halted }
    pub fn total_cycles(&self) -> u64 { self.total_cycles }
}