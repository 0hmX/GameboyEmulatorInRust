use crate::memory_bus::MemoryBus; // Assuming this path is correct

// --- Constants remain the same ---
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
// No longer needs lifetime 'a as memory_bus is passed in methods
pub struct Cpu {
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

    // memory_bus: &'a mut MemoryBus, <-- REMOVED

    ime: bool,        // Interrupt Master Enable flag (true=enabled, false=disabled)
    halted: bool,     // Is the CPU currently halted?
    stop_requested: bool, // Was STOP instruction executed?
    ime_scheduled: bool, // Should IME be enabled after the next instruction?

    total_cycles: u64 // For tracking total cycles executed (useful for timing)
}

// No longer needs lifetime 'a
impl Cpu {
    /// Creates a new CPU instance.
    /// `skip_boot_rom`: If true, initializes registers to post-boot values and PC to 0x0100.
    ///                  If false, initializes for boot ROM execution (PC=0x0000).
    /// **Note:** Does NOT initialize I/O registers if skipping boot ROM. This must be
    ///         done externally by the caller using a MemoryBus reference.
    pub fn new(skip_boot_rom: bool) -> Self {
        // Values based on PanDocs - Post-Boot ROM values
        let (init_a, init_f, init_bc, init_de, init_hl, init_pc) = if skip_boot_rom {
            // Assuming DMG values after boot rom. CGB values differ slightly (A=0x11)
            (0x01, 0xB0, 0x0013, 0x00D8, 0x014D, 0x0100)
        } else {
            // Initial values before boot ROM runs (usually zeroed)
            (0x00, 0x00, 0x0000, 0x0000, 0x0000, 0x0000)
        };

        let cpu = Cpu {
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

            // memory_bus, <-- REMOVED
            ime: false, // IME is disabled initially
            halted: false,
            stop_requested: false,
            ime_scheduled: false, // EI enables IME *after* the next instruction
            total_cycles: 0,
        };

        // IMPORTANT: The I/O register initialization that was here when skip_boot_rom=true
        // has been removed because Cpu::new no longer has access to the MemoryBus.
        // The caller of Cpu::new MUST now perform this initialization separately
        // if skip_boot_rom is true. Example:
        // let mut cpu = Cpu::new(true);
        // initialize_io_registers(&mut memory_bus); // You'd need to write this function

        cpu
    }

    /// Executes a single CPU step: handles interrupts, then fetches and executes an instruction.
    /// Returns the number of **T-cycles** (clock cycles) consumed in this step.
    /// Requires mutable access to the MemoryBus.
    pub fn step(&mut self, memory_bus: &mut MemoryBus) -> u8 {
        // --- Handle pending EI instruction ---
        if self.ime_scheduled {
            self.ime = true;
            self.ime_scheduled = false;
        }

        // --- Check for and Handle Interrupts ---
        let interrupt_cycles = self.handle_interrupts(memory_bus);
        if interrupt_cycles > 0 {
            self.halted = false; // Wake up if halted
            self.stop_requested = false; // Wake up if stopped
            self.total_cycles += interrupt_cycles as u64;
            return interrupt_cycles;
        }

        // --- Handle HALT/STOP state ---
        if self.halted || self.stop_requested {
             self.total_cycles += 4;
             return 4; // Consume 1 M-cycle (4 T-cycles)
        }

        // --- Fetch ---
        // Need to handle potential HALT bug: If HALT bug occurs, PC doesn't increment.
        // Read the instruction *before* checking the halt bug condition for PC handling.
        let opcode_pc = self.pc; // Store PC before fetch for potential halt bug
        let opcode = self.fetch_byte(memory_bus);

        // Check for HALT bug *after* fetching opcode but *before* executing
        let ie = memory_bus.read_byte(IE_REGISTER);
        let iflags = memory_bus.read_byte(IF_REGISTER);
        let halt_bug_condition = self.halted && !self.ime && (ie & iflags & 0x1F) != 0; // Check only enabled+flagged interrupts

        if halt_bug_condition {
            // HALT bug: PC does not increment for the fetched opcode byte.
            self.pc = opcode_pc;
            // The instruction fetched (opcode) will be executed anyway in the next step if still halted.
            // The 'halted' state remains true until an actual interrupt service routine begins.
             println!("WARN: HALT bug triggered! PC kept at {:04X}", self.pc);
             // Fall through to execute NOP-like behaviour (just cycle counting)
             self.total_cycles += 4; // Consume cycles for the "skipped" instruction fetch/decode
             return 4;
        }


        // --- Decode & Execute ---
        let instruction_cycles = self.execute(opcode, memory_bus);
        self.total_cycles += instruction_cycles as u64;

        instruction_cycles
    }


    /// Checks for pending and enabled interrupts and handles the highest priority one.
    /// Returns the number of cycles taken if an interrupt was handled (usually 20), or 0 otherwise.
    fn handle_interrupts(&mut self, memory_bus: &mut MemoryBus) -> u8 {
         // Interrupts are checked even if IME is false *if* the CPU is HALTed
         // because an interrupt can wake the CPU from HALT.
         let check_interrupts = self.ime || self.halted;
         if !check_interrupts {
             return 0;
         }

        // Read Interrupt Flag (IF) and Interrupt Enable (IE) registers
        let if_flags = memory_bus.read_byte(IF_REGISTER);
        let ie_flags = memory_bus.read_byte(IE_REGISTER);

        // Check which interrupts are both requested (IF) and enabled (IE)
        let pending = if_flags & ie_flags & 0x1F; // Mask to relevant 5 bits

        if pending == 0 {
            return 0; // No pending enabled interrupts
        }

        // --- Interrupt pending ---

        // If CPU was HALTed, it wakes up now.
        // The actual interrupt service routine is only run if IME was enabled.
        let was_halted = self.halted;
        self.halted = false; // Wake up regardless

        if !self.ime {
            // Wake from HALT (if halted), but don't service interrupt if IME is off
            return if was_halted { 4 } else { 0 }; // Return cycles for waking? Or just 0? Let step handle cycles. Let's return 0.
        }

        // If we get here, IME must be true. Disable it immediately.
        self.ime = false;
        self.ime_scheduled = false; // Cancel any pending EI

        // Determine highest priority interrupt (lower bit number = higher priority)
        let vector;
        let interrupt_bit;

        if pending & 0x01 != 0 { // VBlank (Priority 0)
            vector = VBLANK_VECTOR; interrupt_bit = 0;
        } else if pending & 0x02 != 0 { // LCD STAT (Priority 1)
            vector = LCD_STAT_VECTOR; interrupt_bit = 1;
        } else if pending & 0x04 != 0 { // Timer (Priority 2)
            vector = TIMER_VECTOR; interrupt_bit = 2;
        } else if pending & 0x08 != 0 { // Serial (Priority 3)
            vector = SERIAL_VECTOR; interrupt_bit = 3;
        } else if pending & 0x10 != 0 { // Joypad (Priority 4)
            vector = JOYPAD_VECTOR; interrupt_bit = 4;
        } else {
            unreachable!("Pending was > 0 but no specific bit found?");
        }

        // Reset the corresponding bit in the IF register
        let current_if = memory_bus.read_byte(IF_REGISTER);
        memory_bus.write_byte(IF_REGISTER, current_if & !(1 << interrupt_bit));

        // Interrupt Handling Sequence takes 5 M-cycles (20 T-cycles):
        // 2 M-cycles delay/internal processing (simulated by cycle count)
        // 2 M-cycles pushing PC high byte, then low byte
        // 1 M-cycle jumping to the vector address
        // Needs the memory_bus for push_word
        self.push_word(self.pc, memory_bus);
        self.pc = vector;

        20 // Cycles consumed by handling the interrupt
    }


    /// Fetches the next byte from memory at the PC and increments the PC.
    fn fetch_byte(&mut self, memory_bus: &mut MemoryBus) -> u8 {
        let byte = memory_bus.read_byte(self.pc);
        // Handle HALT bug - PC increment might be skipped by caller (`step`)
        // So we increment it here normally. `step` will undo if needed.
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    /// Fetches the next word (16 bits, little-endian) from memory at the PC
    /// and increments the PC by 2.
    fn fetch_word(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let low = self.fetch_byte(memory_bus) as u16;
        let high = self.fetch_byte(memory_bus) as u16;
        (high << 8) | low
    }

    /// Pushes a 16-bit value onto the stack.
    fn push_word(&mut self, value: u16, memory_bus: &mut MemoryBus) {
        self.sp = self.sp.wrapping_sub(1);
        memory_bus.write_byte(self.sp, (value >> 8) as u8); // High byte
        self.sp = self.sp.wrapping_sub(1);
        memory_bus.write_byte(self.sp, (value & 0xFF) as u8); // Low byte
    }

    /// Pops a 16-bit value from the stack.
    fn pop_word(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let low = memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (high << 8) | low
    }

    // --- Conditional Jump/Call/Return Helpers ---

    fn jp_cc(&mut self, condition: bool, memory_bus: &mut MemoryBus) -> u8 {
        let jump_addr = self.fetch_word(memory_bus); // Always fetches the address
        if condition {
            self.pc = jump_addr;
            16 // 4 M-cycles (fetch + execute jump)
        } else {
            12 // 3 M-cycles (fetch only)
        }
    }

    // DAA does not access memory
    fn daa(&mut self) {
        let mut adjustment = 0u8;
        let mut set_carry = false;
        let n_flag = self.get_flag(FLAG_N);
        let h_flag = self.get_flag(FLAG_H);
        let c_flag = self.get_flag(FLAG_C);

        if !n_flag { // Addition
            if h_flag || (self.a & 0x0F) > 9 { adjustment |= 0x06; }
            if c_flag || self.a > 0x99 { adjustment |= 0x60; set_carry = true; }
            self.a = self.a.wrapping_add(adjustment);
        } else { // Subtraction
            if h_flag { adjustment |= 0x06; }
            if c_flag { adjustment |= 0x60; set_carry = true; }
            self.a = self.a.wrapping_sub(adjustment);
        }

        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, set_carry || c_flag); // Preserve original carry if subtract caused borrow adjustment
    }

    fn ret_cc(&mut self, condition: bool, memory_bus: &mut MemoryBus) -> u8 {
        if condition {
            self.pc = self.pop_word(memory_bus);
            20 // 5 M-cycles
        } else {
            8 // 2 M-cycles
        }
    }

    fn call_cc(&mut self, condition: bool, memory_bus: &mut MemoryBus) -> u8 {
        let jump_addr = self.fetch_word(memory_bus);
        if condition {
            self.push_word(self.pc, memory_bus);
            self.pc = jump_addr;
            24 // 6 M-cycles
        } else {
            12 // 3 M-cycles
        }
    }

    fn rst(&mut self, vector_offset: u16, memory_bus: &mut MemoryBus) -> u8 {
        self.push_word(self.pc, memory_bus);
        self.pc = vector_offset;
        16 // 4 M-cycles
    }

    // Fetches immediate byte
    fn add_sp_i8(&mut self, memory_bus: &mut MemoryBus) {
        let offset = self.fetch_byte(memory_bus) as i8;
        let value = offset as i16 as u16;
        let sp = self.sp;
        let result = sp.wrapping_add(value);

        let carry = (sp & 0x00FF) + (value & 0x00FF) > 0x00FF;
        let half_carry = (sp & 0x000F) + (value & 0x000F) > 0x000F;

        self.sp = result;
        self.set_flag(FLAG_Z | FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
    }

     // Fetches immediate byte
     fn ld_hl_sp_i8(&mut self, memory_bus: &mut MemoryBus) {
        let offset = self.fetch_byte(memory_bus) as i8;
        let value = offset as i16 as u16;
        let sp = self.sp;
        let result = sp.wrapping_add(value);

        let carry = (sp & 0x00FF) + (value & 0x00FF) > 0x00FF;
        let half_carry = (sp & 0x000F) + (value & 0x000F) > 0x000F;

        self.set_hl(result);
        self.set_flag(FLAG_Z | FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
    }

    /// Decodes and executes a fetched opcode.
    /// Returns the number of T-cycles the instruction took.
    fn execute(&mut self, opcode: u8, memory_bus: &mut MemoryBus) -> u8 {
        match opcode {
            // NOP
            0x00 => 4,

            // LD BC, d16
            0x01 => { let value = self.fetch_word(memory_bus); self.set_bc(value); 12 }
            // LD (BC), A
            0x02 => { memory_bus.write_byte(self.get_bc(), self.a); 8 },
            // INC BC
            0x03 => { self.set_bc(self.get_bc().wrapping_add(1)); 8 },
            // INC B
            0x04 => { self.b = self.inc_u8(self.b); 4 },
            // DEC B
            0x05 => { self.b = self.dec_u8(self.b); 4 },
             // LD B, d8
             0x06 => { self.b = self.fetch_byte(memory_bus); 8 },
            // RLCA
            0x07 => { self.a = self.rlc(self.a); self.set_flag(FLAG_Z, false); 4 },

            // LD (a16), SP
            0x08 => {
                 let addr = self.fetch_word(memory_bus);
                 // Write low byte first, then high byte for SP store
                 memory_bus.write_byte(addr, (self.sp & 0xFF) as u8);
                 memory_bus.write_byte(addr.wrapping_add(1), (self.sp >> 8) as u8);
                 20
             },
            // ADD HL, BC
            0x09 => { let val = self.get_bc(); self.add_hl(val); 8 },
             // LD A, (BC)
             0x0A => { self.a = memory_bus.read_byte(self.get_bc()); 8 },
             // DEC BC
             0x0B => { self.set_bc(self.get_bc().wrapping_sub(1)); 8 },
             // INC C
             0x0C => { self.c = self.inc_u8(self.c); 4 },
             // DEC C
             0x0D => { self.c = self.dec_u8(self.c); 4 },
             // LD C, d8
             0x0E => { self.c = self.fetch_byte(memory_bus); 8 },
            // RRCA
            0x0F => { self.a = self.rrc(self.a); self.set_flag(FLAG_Z, false); 4 },


            // STOP
            0x10 => {
                 // STOP consumes the next byte (usually 0x00) but does nothing with it
                 // fetch_byte handles the PC increment
                 let _ = self.fetch_byte(memory_bus); // Consume the 0x00
                 // TODO: Handle CGB speed switching if applicable.
                 self.stop_requested = true;
                 // Actual low power mode isn't simulated, just stops fetching.
                 4 // STOP instruction itself takes 4 cycles
             },
            // LD DE, d16
            0x11 => { let value = self.fetch_word(memory_bus); self.set_de(value); 12 },
             // LD (DE), A
             0x12 => { memory_bus.write_byte(self.get_de(), self.a); 8 },
             // INC DE
             0x13 => { self.set_de(self.get_de().wrapping_add(1)); 8 },
            // INC D
            0x14 => { self.d = self.inc_u8(self.d); 4 },
            // DEC D
            0x15 => { self.d = self.dec_u8(self.d); 4 },
             // LD D, d8
             0x16 => { self.d = self.fetch_byte(memory_bus); 8 },
            // RLA
            0x17 => { self.a = self.rl(self.a); self.set_flag(FLAG_Z, false); 4 },

            // JR r8
            0x18 => self.jr_cc(true, memory_bus),
            // ADD HL, DE
            0x19 => { let val = self.get_de(); self.add_hl(val); 8 },
             // LD A, (DE)
             0x1A => { self.a = memory_bus.read_byte(self.get_de()); 8 },
             // DEC DE
             0x1B => { self.set_de(self.get_de().wrapping_sub(1)); 8 },
            // INC E
            0x1C => { self.e = self.inc_u8(self.e); 4 },
            // DEC E
            0x1D => { self.e = self.dec_u8(self.e); 4 },
            // LD E, d8
            0x1E => { self.e = self.fetch_byte(memory_bus); 8 },
            // RRA
            0x1F => { self.a = self.rr(self.a); self.set_flag(FLAG_Z, false); 4 },


            // JR NZ, r8
            0x20 => self.jr_cc(!self.get_flag(FLAG_Z), memory_bus),
            // LD HL, d16
            0x21 => { let val = self.fetch_word(memory_bus); self.set_hl(val); 12 },
            // LD (HL+), A
            0x22 => {
                 let addr = self.get_hl();
                 memory_bus.write_byte(addr, self.a);
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
             0x26 => { self.h = self.fetch_byte(memory_bus); 8 },
            // DAA
            0x27 => { self.daa(); 4 },

            // JR Z, r8
            0x28 => self.jr_cc(self.get_flag(FLAG_Z), memory_bus),
            // ADD HL, HL
            0x29 => { let val = self.get_hl(); self.add_hl(val); 8 },
            // LD A, (HL+)
            0x2A => {
                 let addr = self.get_hl();
                 self.a = memory_bus.read_byte(addr);
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
             0x2E => { self.l = self.fetch_byte(memory_bus); 8 },
            // CPL
            0x2F => {
                self.a = !self.a;
                self.set_flag(FLAG_N | FLAG_H, true);
                4
            },

            // JR NC, r8
            0x30 => self.jr_cc(!self.get_flag(FLAG_C), memory_bus),
            // LD SP, d16
            0x31 => { self.sp = self.fetch_word(memory_bus); 12 },
             // LD (HL-), A
             0x32 => {
                 let addr = self.get_hl();
                 memory_bus.write_byte(addr, self.a);
                 self.set_hl(addr.wrapping_sub(1));
                 8
            },
             // INC SP
             0x33 => { self.sp = self.sp.wrapping_add(1); 8 },
            // INC (HL)
            0x34 => {
                let addr = self.get_hl();
                let value = memory_bus.read_byte(addr);
                let result = self.inc_u8(value);
                memory_bus.write_byte(addr, result);
                12
            },
            // DEC (HL)
            0x35 => {
                let addr = self.get_hl();
                let value = memory_bus.read_byte(addr);
                let result = self.dec_u8(value);
                memory_bus.write_byte(addr, result);
                12
            },
            // LD (HL), d8
            0x36 => {
                let value = self.fetch_byte(memory_bus);
                memory_bus.write_byte(self.get_hl(), value);
                12
            },
            // SCF
            0x37 => {
                self.set_flag(FLAG_N | FLAG_H, false);
                self.set_flag(FLAG_C, true);
                4
            },

            // JR C, r8
            0x38 => self.jr_cc(self.get_flag(FLAG_C), memory_bus),
            // ADD HL, SP
            0x39 => { let val = self.sp; self.add_hl(val); 8 },
            // LD A, (HL-)
            0x3A => {
                 let addr = self.get_hl();
                 self.a = memory_bus.read_byte(addr);
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
            0x3E => { self.a = self.fetch_byte(memory_bus); 8 },
            // CCF
            0x3F => {
                let current_c = self.get_flag(FLAG_C);
                self.set_flag(FLAG_N | FLAG_H, false);
                self.set_flag(FLAG_C, !current_c);
                4
            },

             // LD r8, r8' (Includes LD B,B etc. which are 4 cycles)
             // Also includes LD r, (HL) (8 cycles) and LD (HL), r (8 cycles)
             // HALT (0x76) is handled separately below.
             0x40..=0x7F => {
                 if opcode == 0x76 {
                     self.halt(memory_bus) // Pass memory_bus for HALT bug check
                 } else {
                     self.ld_r8_r8(opcode, memory_bus)
                 }
                 // Cycles are returned by ld_r8_r8 or halt
             },

             // HALT (Moved check into 0x40..=0x7F range handler)
             // 0x76 => self.halt(memory_bus), <-- Handled above

             // --- The LD r, (HL) and LD (HL), r cases are handled within ld_r8_r8 ---
             // 0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x7E => { ... handled by ld_r8_r8 ... }
             // 0x70..=0x75 | 0x77 => { ... handled by ld_r8_r8 ... }


            // --- ALU Operations: ADD/ADC/SUB/SBC/AND/XOR/OR/CP A, r8|(HL) ---
            0x80..=0xBF => {
                // Determine operand source and if it's (HL)
                let operand_code = opcode & 0x07;
                let is_hl_operand = operand_code == 0x06;
                let operand = self.get_alu_operand(operand_code, memory_bus);

                match (opcode >> 3) & 0x07 {
                    0 => self.add_a(operand, false), // ADD A, operand
                    1 => self.add_a(operand, true),  // ADC A, operand
                    2 => self.sub_a(operand, false), // SUB A, operand
                    3 => self.sub_a(operand, true),  // SBC A, operand
                    4 => self.and_a(operand),        // AND A, operand
                    5 => self.xor_a(operand),        // XOR A, operand
                    6 => self.or_a(operand),         // OR A, operand
                    7 => self.cp_a(operand),         // CP A, operand
                    _ => unreachable!(),
                }
                if is_hl_operand { 8 } else { 4 } // Base cycles
            },


            // --- Conditional Returns ---
            0xC0 => self.ret_cc(!self.get_flag(FLAG_Z), memory_bus), // RET NZ
            0xC1 => { let val = self.pop_word(memory_bus); self.set_bc(val); 12 }, // POP BC
            0xC2 => self.jp_cc(!self.get_flag(FLAG_Z), memory_bus), // JP NZ, nn
            0xC3 => { self.pc = self.fetch_word(memory_bus); 16 }, // JP nn
            0xC4 => self.call_cc(!self.get_flag(FLAG_Z), memory_bus), // CALL NZ, nn
            0xC5 => { self.push_word(self.get_bc(), memory_bus); 16 }, // PUSH BC
            0xC6 => { let v = self.fetch_byte(memory_bus); self.add_a(v, false); 8 }, // ADD A, n8
            0xC7 => self.rst(0x00, memory_bus), // RST 00H

            0xC8 => self.ret_cc(self.get_flag(FLAG_Z), memory_bus), // RET Z
            0xC9 => { self.pc = self.pop_word(memory_bus); 16 }, // RET
            0xCA => self.jp_cc(self.get_flag(FLAG_Z), memory_bus), // JP Z, nn
            // CB Prefix
            0xCB => {
                let cb_opcode = self.fetch_byte(memory_bus);
                self.execute_cb(cb_opcode, memory_bus) // execute_cb returns cycles
            },
            0xCC => self.call_cc(self.get_flag(FLAG_Z), memory_bus), // CALL Z, nn
            0xCD => { // CALL nn
                let addr = self.fetch_word(memory_bus);
                self.push_word(self.pc, memory_bus);
                self.pc = addr;
                24
            },
            0xCE => { let v = self.fetch_byte(memory_bus); self.add_a(v, true); 8 }, // ADC A, n8
            0xCF => self.rst(0x08, memory_bus), // RST 08H


            0xD0 => self.ret_cc(!self.get_flag(FLAG_C), memory_bus), // RET NC
            0xD1 => { let val = self.pop_word(memory_bus); self.set_de(val); 12 }, // POP DE
            0xD2 => self.jp_cc(!self.get_flag(FLAG_C), memory_bus), // JP NC, nn
            0xD3 => { panic!("Invalid opcode: 0xD3 at {:04X}", self.pc.wrapping_sub(1)); } // Invalid
            0xD4 => self.call_cc(!self.get_flag(FLAG_C), memory_bus), // CALL NC, nn
            0xD5 => { self.push_word(self.get_de(), memory_bus); 16 }, // PUSH DE
            0xD6 => { let v = self.fetch_byte(memory_bus); self.sub_a(v, false); 8 }, // SUB A, n8
            0xD7 => self.rst(0x10, memory_bus), // RST 10H

            0xD8 => self.ret_cc(self.get_flag(FLAG_C), memory_bus), // RET C
            0xD9 => { // RETI
                self.pc = self.pop_word(memory_bus);
                self.ime = true; // Enable interrupts AFTER executing RETI
                // Note: This differs from EI which enables after the *next* instruction.
                self.ime_scheduled = false; // Ensure no pending EI schedule interferes
                16
            },
            0xDA => self.jp_cc(self.get_flag(FLAG_C), memory_bus), // JP C, nn
            0xDB => { panic!("Invalid opcode: 0xDB at {:04X}", self.pc.wrapping_sub(1)); } // Invalid
            0xDC => self.call_cc(self.get_flag(FLAG_C), memory_bus), // CALL C, nn
            0xDD => { panic!("Invalid opcode: 0xDD at {:04X}", self.pc.wrapping_sub(1)); } // Invalid
            0xDE => { let v = self.fetch_byte(memory_bus); self.sub_a(v, true); 8 }, // SBC A, n8
            0xDF => self.rst(0x18, memory_bus), // RST 18H


            // LDH (a8), A --- Write A to 0xFF00 + n8
            0xE0 => {
                let offset = self.fetch_byte(memory_bus) as u16;
                memory_bus.write_byte(0xFF00 + offset, self.a);
                12
            },
            // POP HL
            0xE1 => { let val = self.pop_word(memory_bus); self.set_hl(val); 12 },
             // LD (C), A --- Write A to 0xFF00 + C
             0xE2 => {
                memory_bus.write_byte(0xFF00 + self.c as u16, self.a);
                8
            },
            0xE3 | 0xE4 => { panic!("Invalid opcode: {:02X} at {:04X}", opcode, self.pc.wrapping_sub(1)); } // Invalid
            // PUSH HL
            0xE5 => { self.push_word(self.get_hl(), memory_bus); 16 },
            // AND A, n8
            0xE6 => { let v = self.fetch_byte(memory_bus); self.and_a(v); 8 },
            // RST 20H
            0xE7 => self.rst(0x20, memory_bus),

            // ADD SP, r8
            0xE8 => { self.add_sp_i8(memory_bus); 16 },
            // JP HL
            0xE9 => { self.pc = self.get_hl(); 4 },
            // LD (a16), A
            0xEA => {
                let addr = self.fetch_word(memory_bus);
                memory_bus.write_byte(addr, self.a);
                16
            },
            0xEB | 0xEC | 0xED => { panic!("Invalid opcode: {:02X} at {:04X}", opcode, self.pc.wrapping_sub(1)); } // Invalid
            // XOR A, n8
            0xEE => { let v = self.fetch_byte(memory_bus); self.xor_a(v); 8 },
            // RST 28H
            0xEF => self.rst(0x28, memory_bus),


            // LDH A, (a8) --- Read from 0xFF00 + n8 into A
            0xF0 => {
                let offset = self.fetch_byte(memory_bus) as u16;
                self.a = memory_bus.read_byte(0xFF00 + offset);
                12
            },
            // POP AF
            0xF1 => { let val = self.pop_word(memory_bus); self.set_af(val); 12 },
            // LD A, (C) --- Read from 0xFF00 + C into A
            0xF2 => {
                 self.a = memory_bus.read_byte(0xFF00 + self.c as u16);
                 8
             },
            // DI
            0xF3 => {
                self.ime = false;
                self.ime_scheduled = false; // Cancel pending EI
                4
            },
            0xF4 => { panic!("Invalid opcode: 0xF4 at {:04X}", self.pc.wrapping_sub(1)); } // Invalid
            // PUSH AF
            0xF5 => { self.push_word(self.get_af(), memory_bus); 16 },
            // OR A, n8
            0xF6 => { let v = self.fetch_byte(memory_bus); self.or_a(v); 8 },
            // RST 30H
            0xF7 => self.rst(0x30, memory_bus),

            // LD HL, SP+r8
            0xF8 => { self.ld_hl_sp_i8(memory_bus); 12 },
            // LD SP, HL
            0xF9 => { self.sp = self.get_hl(); 8 },
            // LD A, (a16)
            0xFA => {
                 let addr = self.fetch_word(memory_bus);
                 self.a = memory_bus.read_byte(addr);
                 16
            },
            // EI
            0xFB => {
                // IME is enabled *after* the instruction following EI
                self.ime_scheduled = true;
                4
            },
            0xFC | 0xFD => { panic!("Invalid opcode: {:02X} at {:04X}", opcode, self.pc.wrapping_sub(1)); } // Invalid
            // CP A, n8
            0xFE => {
                 let value = self.fetch_byte(memory_bus);
                 self.cp_a(value);
                 8
            },
            // RST 38H
            0xFF => self.rst(0x38, memory_bus),

            // _ => { ... } // Should be covered by ranges now
        }
    }

    /// Executes CB-prefixed opcodes.
    fn execute_cb(&mut self, opcode: u8, memory_bus: &mut MemoryBus) -> u8 {
         let target_reg_code = opcode & 0x07; // 0-5: B,C,D,E,H,L, 6: (HL), 7: A
         let operation_group = opcode >> 6; // 0: Rotate/Shift, 1: BIT, 2: RES, 3: SET
         let operation_subcode = (opcode >> 3) & 0x07; // Specific rotate/shift type or bit index

         let mut cycles = 8; // Base cycles for register ops
         let is_hl_operand = target_reg_code == 6;

         // Read the source value
         let value = match target_reg_code {
             0 => self.b, 1 => self.c, 2 => self.d, 3 => self.e,
             4 => self.h, 5 => self.l,
             6 => {
                 cycles = match operation_group {
                     1 => 12, // BIT (HL)
                     _ => 16, // RLC/RRC/RL/RR/SLA/SRA/SWAP/SRL/RES/SET (HL)
                 };
                 memory_bus.read_byte(self.get_hl())
             },
             7 => self.a,
             _ => unreachable!(),
         };

         // Perform the operation
         let result = match operation_group {
             0 => { // Rotates/Shifts
                 match operation_subcode {
                     0 => self.rlc(value), 1 => self.rrc(value), 2 => self.rl(value), 3 => self.rr(value),
                     4 => self.sla(value), 5 => self.sra(value), 6 => self.swap(value), 7 => self.srl(value),
                     _ => unreachable!(),
                 }
             },
             1 => { // BIT b, r/(HL)
                 self.op_bit(operation_subcode, value);
                 value // BIT doesn't modify the value, only flags
             }
             2 => { // RES b, r/(HL)
                 value & !(1 << operation_subcode)
             }
              3 => { // SET b, r/(HL)
                 value | (1 << operation_subcode)
             }
            _ => unreachable!(),
         };

        // Write result back if it wasn't a BIT operation
        if operation_group != 1 { // If not BIT
             match target_reg_code {
                 0 => self.b = result, 1 => self.c = result, 2 => self.d = result, 3 => self.e = result,
                 4 => self.h = result, 5 => self.l = result,
                 6 => memory_bus.write_byte(self.get_hl(), result),
                 7 => self.a = result,
                 _ => unreachable!(),
             };
        }

        cycles
    }

    // --- Register Getters/Setters (Combined) --- No changes needed ---
    fn get_af(&self) -> u16 { ((self.a as u16) << 8) | (self.f as u16 & 0xF0) } // Mask low bits of F on read
    fn set_af(&mut self, value: u16) { self.a = (value >> 8) as u8; self.f = (value & 0x00F0) as u8; } // Mask low bits on write
    fn get_bc(&self) -> u16 { ((self.b as u16) << 8) | (self.c as u16) }
    fn set_bc(&mut self, value: u16) { self.b = (value >> 8) as u8; self.c = (value & 0x00FF) as u8; }
    fn get_de(&self) -> u16 { ((self.d as u16) << 8) | (self.e as u16) }
    fn set_de(&mut self, value: u16) { self.d = (value >> 8) as u8; self.e = (value & 0x00FF) as u8; }
    fn get_hl(&self) -> u16 { ((self.h as u16) << 8) | (self.l as u16) }
    fn set_hl(&mut self, value: u16) { self.h = (value >> 8) as u8; self.l = (value & 0x00FF) as u8; }

    // --- Flag Manipulation Helpers --- No changes needed ---
    fn set_flag(&mut self, flag_mask: u8, set: bool) {
        if set { self.f |= flag_mask; } else { self.f &= !flag_mask; }
        self.f &= 0xF0; // Ensure lower bits are always zero
    }
    fn get_flag(&self, flag_mask: u8) -> bool { (self.f & flag_mask) != 0 }

     // --- CPU State Control ---
     // Takes memory_bus to check for HALT bug condition
     fn halt(&mut self, memory_bus: &MemoryBus) -> u8 {
        let ie = memory_bus.read_byte(IE_REGISTER);
        let iflags = memory_bus.read_byte(IF_REGISTER);
        // HALT bug condition: IME=0 and IE & IF has pending interrupts
        if !self.ime && (ie & iflags & 0x1F) != 0 {
            // HALT bug triggered. Don't set self.halted = true;
            // PC increment is handled in `step` by checking this condition again.
             println!("WARN: HALT executed with IME=0 and pending interrupt (IE&IF={:02X}). HALT bug behavior.", ie & iflags);
            // No actual halt occurs, instruction proceeds (but PC doesn't increment correctly - handled in step)
        } else {
            self.halted = true;
        }
        4 // HALT instruction itself takes 4 cycles
    }

    // --- Instruction Helpers ---

    // JR cc, r8 (fetches immediate byte)
    fn jr_cc(&mut self, condition: bool, memory_bus: &mut MemoryBus) -> u8 {
        let relative_offset = self.fetch_byte(memory_bus) as i8;
        if condition {
            let current_pc = self.pc;
            self.pc = current_pc.wrapping_add(relative_offset as i16 as u16);
            12 // 3 M-cycles
        } else {
            8 // 2 M-cycles
        }
    }

    // INC r8 (no memory access)
    fn inc_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, (value & 0x0F) == 0x0F); // Half carry if lower nibble was F
        // C unchanged
        result
    }

    // DEC r8 (no memory access)
    fn dec_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, (value & 0x0F) == 0x00); // Half borrow if lower nibble was 0
        // C unchanged
        result
    }

     // ADD HL, rr (no memory access)
     fn add_hl(&mut self, value: u16) {
        let hl = self.get_hl();
        let (result, carry) = hl.overflowing_add(value);
        let half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;

        self.set_hl(result);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
     }


    // LD r8, r8' / LD r, (HL) / LD (HL), r
    fn ld_r8_r8(&mut self, opcode: u8, memory_bus: &mut MemoryBus) -> u8 {
        let source_reg_code = opcode & 0x07;
        let dest_reg_code = (opcode >> 3) & 0x07;

        let is_source_hl = source_reg_code == 6;
        let is_dest_hl = dest_reg_code == 6;

        let value = if is_source_hl {
            memory_bus.read_byte(self.get_hl())
        } else {
            match source_reg_code {
                 0 => self.b, 1 => self.c, 2 => self.d, 3 => self.e,
                 4 => self.h, 5 => self.l, 7 => self.a,
                 _ => unreachable!(),
            }
        };

         if is_dest_hl {
            memory_bus.write_byte(self.get_hl(), value);
         } else {
             match dest_reg_code {
                 0 => self.b = value, 1 => self.c = value, 2 => self.d = value, 3 => self.e = value,
                 4 => self.h = value, 5 => self.l = value, 7 => self.a = value,
                 _ => unreachable!(),
             };
         }

         if is_source_hl || is_dest_hl { 8 } else { 4 }
    }

    // Helper to get operand for ALU operations (handles (HL) read)
    fn get_alu_operand(&self, operand_code: u8, memory_bus: &MemoryBus) -> u8 {
        match operand_code {
            0 => self.b, 1 => self.c, 2 => self.d, 3 => self.e,
            4 => self.h, 5 => self.l, 6 => memory_bus.read_byte(self.get_hl()), 7 => self.a,
             _ => unreachable!(),
        }
    }


     // --- Actual ALU operations --- No changes needed (no memory access) ---
     fn add_a(&mut self, value: u8, use_carry: bool) {
        let carry_in = if use_carry && self.get_flag(FLAG_C) { 1 } else { 0 };
        let (res1, c1) = self.a.overflowing_add(value);
        let (result, c2) = res1.overflowing_add(carry_in);
        let carry_out = c1 || c2;
        let half_carry = (self.a & 0x0F) + (value & 0x0F) + carry_in > 0x0F;

        self.a = result;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry_out);
    }

     fn sub_a(&mut self, value: u8, use_carry: bool) {
        let carry_in = if use_carry && self.get_flag(FLAG_C) { 1 } else { 0 };
        let (res1, b1) = self.a.overflowing_sub(value);
        let (result, b2) = res1.overflowing_sub(carry_in);
        let borrow_out = b1 || b2;
        let half_borrow = (self.a & 0x0F) < (value & 0x0F) + carry_in;

        self.a = result;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, half_borrow);
        self.set_flag(FLAG_C, borrow_out);
    }

    fn and_a(&mut self, value: u8) {
        self.a &= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true);
        self.set_flag(FLAG_C, false);
    }

    fn xor_a(&mut self, value: u8) {
        self.a ^= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
    }

    fn or_a(&mut self, value: u8) {
        self.a |= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
    }

     fn cp_a(&mut self, value: u8) {
        let temp_a = self.a;
        self.sub_a(value, false); // Use sub logic for flags
        self.a = temp_a; // Restore A
     }


    // --- CB Prefix Operations --- No changes needed (no memory access) ---
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
         let result = value.rotate_left(4); // Swaps upper and lower nibbles
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
         // C unchanged
     }

    // --- Public accessors --- No changes needed ---
    pub fn pc(&self) -> u16 { self.pc }
    pub fn sp(&self) -> u16 { self.sp }
    pub fn registers(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8) { (self.a, self.f & 0xF0, self.b, self.c, self.d, self.e, self.h, self.l) }
    pub fn ime(&self) -> bool { self.ime }
    pub fn halted(&self) -> bool { self.halted }
    pub fn total_cycles(&self) -> u64 { self.total_cycles }

    /// Helper to initialize I/O registers after CPU creation when skipping boot ROM.
    /// This is needed because `Cpu::new` no longer takes a MemoryBus.
    /// Call this *after* creating the CPU and *before* starting the execution loop.
    pub fn initialize_post_boot_io(memory_bus: &mut MemoryBus) {
        // Essential initial I/O register values (DMG)
        memory_bus.write_byte(0xFF05, 0x00); // TIMA
        memory_bus.write_byte(0xFF06, 0x00); // TMA
        memory_bus.write_byte(0xFF07, 0x00); // TAC
        memory_bus.write_byte(0xFF10, 0x80); // NR10
        memory_bus.write_byte(0xFF11, 0xBF); // NR11
        memory_bus.write_byte(0xFF12, 0xF3); // NR12
        memory_bus.write_byte(0xFF14, 0xBF); // NR14
        memory_bus.write_byte(0xFF16, 0x3F); // NR21
        memory_bus.write_byte(0xFF17, 0x00); // NR22
        memory_bus.write_byte(0xFF19, 0xBF); // NR24
        memory_bus.write_byte(0xFF1A, 0x7F); // NR30
        memory_bus.write_byte(0xFF1B, 0xFF); // NR31
        memory_bus.write_byte(0xFF1C, 0x9F); // NR32
        memory_bus.write_byte(0xFF1E, 0xBF); // NR33
        memory_bus.write_byte(0xFF20, 0xFF); // NR41
        memory_bus.write_byte(0xFF21, 0x00); // NR42
        memory_bus.write_byte(0xFF22, 0x00); // NR43
        memory_bus.write_byte(0xFF23, 0xBF); // NR44
        memory_bus.write_byte(0xFF24, 0x77); // NR50
        memory_bus.write_byte(0xFF25, 0xF3); // NR51
        // NOTE: NR52 value differs slightly between DMG (F1) and SGB (F0)
        memory_bus.write_byte(0xFF26, 0xF1); // NR52 - F1 for DMG
        memory_bus.write_byte(0xFF40, 0x91); // LCDC
        // STAT initial state - mode often depends on timing, 0x85 is a common post-boot value
        memory_bus.write_byte(0xFF41, 0x85); // STAT
        memory_bus.write_byte(0xFF42, 0x00); // SCY
        memory_bus.write_byte(0xFF43, 0x00); // SCX
        memory_bus.write_byte(0xFF44, 0x00); // LY - Should start at 0, will increment
        memory_bus.write_byte(0xFF45, 0x00); // LYC
        memory_bus.write_byte(0xFF47, 0xFC); // BGP
        memory_bus.write_byte(0xFF48, 0xFF); // OBP0
        memory_bus.write_byte(0xFF49, 0xFF); // OBP1
        memory_bus.write_byte(0xFF4A, 0x00); // WY
        memory_bus.write_byte(0xFF4B, 0x00); // WX
        memory_bus.write_byte(IE_REGISTER, 0x00);  // IE
        // IF often starts non-zero (e.g., 0xE1) after boot ROM, indicating VBLANK occurred.
        // Starting clean might be simpler for emulation unless precise boot is needed.
        memory_bus.write_byte(IF_REGISTER, 0x00); // IF (starting clean)
        // Write 0x01 to 0xFF50 to disable boot ROM mapping (MemoryBus usually handles this logic)
        memory_bus.write_byte(0xFF50, 0x01);
    }
}