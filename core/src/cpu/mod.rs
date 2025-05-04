//! The Sharp SM83 CPU core implementation.

use crate::memory_bus::MemoryBus;
use crate::memory_map; // Use qualified paths for memory map constants
use crate::memory_map::{
    JOYPAD_INTERRUPT_BIT, LCD_STAT_INTERRUPT_BIT, SERIAL_INTERRUPT_BIT, TIMER_INTERRUPT_BIT,
    VBLANK_INTERRUPT_BIT,
};
use instruction::{CB_INSTRUCTIONS, INSTRUCTIONS};
use log;

// Declare submodules
mod constants;
#[macro_use]
mod ops_macros;
mod instruction;
mod ops_alu;
mod ops_cb;
mod ops_control;
mod ops_load;
mod ops_rot_shift;

// Re-export public constants if needed by external modules
pub use constants::*;

// Type alias for CPU operation results
pub type CpuResult<T> = Result<T, String>;

/// Represents the Game Boy's SM83 CPU state and provides execution logic.
#[derive(Debug, Clone)]
pub struct Cpu {
    // --- 8-bit Registers ---
    a: u8, // Accumulator
    f: u8, // Flags (ZNHC----)
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,

    // --- 16-bit Registers ---
    sp: u16, // Stack Pointer
    pc: u16, // Program Counter

    // --- CPU State Flags ---
    ime: bool,            // Interrupt Master Enable flag (enabled/disabled)
    halted: bool,         // CPU is in HALT state (waiting for interrupt)
    stop_requested: bool, // CPU received STOP instruction (low power state)
    ime_scheduled: bool,  // IME will be enabled after the next instruction

    // --- Internal Timing/Execution State ---
    total_cycles: u64,   // Total T-cycles executed since start/reset
    fetched_opcode: u8,  // Last opcode fetched (for error reporting/debugging)
    instruction_pc: u16, // PC at the start of the current instruction (for reads/debugging)
}

// Core CPU logic (new, step, interrupts, helpers, accessors) remains here
impl Cpu {
    /// Creates a new CPU instance, optionally skipping the boot ROM sequence.
    pub fn new(skip_boot_rom: bool) -> Self {
        // Initial register values depend on whether the boot ROM is executed.
        // These values are set *after* the boot ROM finishes (if executed).
        let (init_a, init_f, init_bc, init_de, init_hl, init_pc, init_sp) = if skip_boot_rom {
            // Values based on Pandocs post-boot section for DMG
            (0x01, 0xB0, 0x0013, 0x00D8, 0x014D, 0x0100, 0xFFFE)
        } else {
            // Boot ROM starts execution at 0x0000 with registers typically zeroed
            (0x00, 0x00, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000) // SP init value might vary? Set high later.
        };

        Cpu {
            a: init_a,
            f: init_f & 0xF0, // Ensure lower bits are zero
            b: (init_bc >> 8) as u8,
            c: init_bc as u8,
            d: (init_de >> 8) as u8,
            e: init_de as u8,
            h: (init_hl >> 8) as u8,
            l: init_hl as u8,
            sp: init_sp,
            pc: init_pc,
            ime: false, // IME is initially disabled
            halted: false,
            stop_requested: false,
            ime_scheduled: false,
            total_cycles: 0,
            fetched_opcode: 0,
            instruction_pc: 0,
        }
    }

    /// Initializes I/O registers to their state after the boot ROM finishes.
    /// Should only be called if `skip_boot_rom` was true.
    pub fn initialize_post_boot_io(memory_bus: &mut MemoryBus) {
        // Values based on Pandocs "Power Up Sequence" section, post-DMG boot ROM
        memory_bus.write_byte(memory_map::TIMA_ADDR, 0x00);
        memory_bus.write_byte(memory_map::TMA_ADDR, 0x00);
        memory_bus.write_byte(memory_map::TAC_ADDR, 0x00);
        memory_bus.write_byte(memory_map::NR10_ADDR, 0x80);
        memory_bus.write_byte(memory_map::NR11_ADDR, 0xBF);
        memory_bus.write_byte(memory_map::NR12_ADDR, 0xF3);
        memory_bus.write_byte(memory_map::NR14_ADDR, 0xBF);
        memory_bus.write_byte(memory_map::NR21_ADDR, 0x3F);
        memory_bus.write_byte(memory_map::NR22_ADDR, 0x00);
        memory_bus.write_byte(memory_map::NR24_ADDR, 0xBF);
        memory_bus.write_byte(memory_map::NR30_ADDR, 0x7F);
        memory_bus.write_byte(memory_map::NR31_ADDR, 0xFF);
        memory_bus.write_byte(memory_map::NR32_ADDR, 0x9F);
        memory_bus.write_byte(memory_map::NR34_ADDR, 0xBF); // NR33 is W only
        memory_bus.write_byte(memory_map::NR41_ADDR, 0xFF); // Sound length
        memory_bus.write_byte(memory_map::NR42_ADDR, 0x00);
        memory_bus.write_byte(memory_map::NR43_ADDR, 0x00);
        memory_bus.write_byte(memory_map::NR44_ADDR, 0xBF); // Trigger is Write Only
        memory_bus.write_byte(memory_map::NR50_ADDR, 0x77);
        memory_bus.write_byte(memory_map::NR51_ADDR, 0xF3);
        memory_bus.write_byte(memory_map::NR52_ADDR, 0xF1); // F1 for DMG
        memory_bus.write_byte(memory_map::LCDC_ADDR, 0x91);
        memory_bus.write_byte(memory_map::STAT_ADDR, 0x85); // STAT - Mode 1 + LYC flag? Check boot.
        memory_bus.write_byte(memory_map::SCY_ADDR, 0x00);
        memory_bus.write_byte(memory_map::SCX_ADDR, 0x00);
        memory_bus.write_byte(memory_map::LYC_ADDR, 0x00);
        // LY (0xFF44) is Read Only, driven by PPU, usually 0 post-boot?
        memory_bus.write_byte(memory_map::BGP_ADDR, 0xFC);
        memory_bus.write_byte(memory_map::OBP0_ADDR, 0xFF);
        memory_bus.write_byte(memory_map::OBP1_ADDR, 0xFF);
        memory_bus.write_byte(memory_map::WY_ADDR, 0x00);
        memory_bus.write_byte(memory_map::WX_ADDR, 0x00);
        memory_bus.write_byte(memory_map::INTERRUPT_ENABLE_REGISTER, 0x00);
        // IF (0xFF0F) is often 0xE1 post-boot (VBL, LCD, Timer flags set) but clear here?
        // Let's initialize IF based on the MemoryBus::new() which sets it to E1.
        // memory_bus.write_byte(memory_map::IF_ADDR, 0x00); // Or E1?
        memory_bus.write_byte(0xFF50, 0x01); // Boot ROM Lock (write 1 to disable)
    }

    /// Executes a single CPU instruction cycle (fetch, decode, execute).
    /// Returns the number of T-cycles consumed by the instruction.
    pub fn step(&mut self, memory_bus: &mut MemoryBus) -> CpuResult<u16> {
        // --- Interrupt Handling Phase ---
        let mut ime_just_enabled = false;
        if self.ime_scheduled {
            self.ime = true;
            self.ime_scheduled = false;
            ime_just_enabled = true;
        }

        let interrupt_cycles = if self.ime && !ime_just_enabled {
            self.handle_interrupts(memory_bus)
        } else {
            0
        };

        if interrupt_cycles > 0 {
            self.halted = false;
            self.stop_requested = false;
            self.total_cycles = self.total_cycles.wrapping_add(interrupt_cycles as u64);
            return Ok(interrupt_cycles as u16);
        }

        // --- Halted/Stopped Phase ---
        if self.halted {
            let ie = memory_bus.read_byte(memory_map::INTERRUPT_ENABLE_REGISTER);
            let iflags = memory_bus.read_byte(memory_map::IF_ADDR);
            if (ie & iflags & 0x1F) != 0 {
                self.halted = false;
                self.total_cycles = self.total_cycles.wrapping_add(4);
                return Ok(4); // Wake up takes 1 cycle (4 T-cycles)
            }
        }
        if self.halted || self.stop_requested {
            self.total_cycles = self.total_cycles.wrapping_add(4);
            return Ok(4); // Stay halted/stopped
        }

        // --- Fetch Phase ---
        self.instruction_pc = self.pc;
        self.fetched_opcode = self.read_byte_at_pc(memory_bus);

        // --- Decode Phase ---
        let instruction = &INSTRUCTIONS[self.fetched_opcode as usize];

        // --- Advance PC Phase ---
        self.pc = self.pc.wrapping_add(instruction.length as u16);

        // --- Execute Phase ---
        let execute_result = (instruction.execute)(self, memory_bus);

        // --- Process Result ---
        match execute_result {
            Ok(additional_cycles) => {
                let base_cycles = if self.fetched_opcode == 0xCB {
                    4 // Base cost of CB prefix itself
                } else {
                    instruction.cycles as u16
                };
                let total_instruction_cycles = base_cycles.wrapping_add(additional_cycles);
                self.total_cycles = self
                    .total_cycles
                    .wrapping_add(total_instruction_cycles as u64);
                Ok(total_instruction_cycles)
            }
            Err(error_message) => {
                log::error!(
                    "CPU Error at PC={:#06X} (Opcode {:#04X}): {}",
                    self.instruction_pc,
                    self.fetched_opcode,
                    error_message
                );
                let base_cycles = if self.fetched_opcode == 0xCB {
                    4
                } else {
                    instruction.cycles as u16
                };
                self.total_cycles = self.total_cycles.wrapping_add(base_cycles as u64);
                Err(format!(
                    "CPU Error at PC={:#06X} (Opcode {:#04X}): {}",
                    self.instruction_pc, self.fetched_opcode, error_message
                ))
            }
        }
    }

    /// Checks for and handles pending interrupts if IME is enabled.
    /// Returns the number of cycles taken if an interrupt was handled (20), otherwise 0.
    fn handle_interrupts(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let if_flags = memory_bus.read_byte(memory_map::IF_ADDR);
        let ie_flags = memory_bus.read_byte(memory_map::INTERRUPT_ENABLE_REGISTER);
        let pending = if_flags & ie_flags & 0x1F;
        if pending == 0 {
            return 0;
        }

        self.ime = false;
        self.ime_scheduled = false;

        let (vector, interrupt_bit) = if pending & (1 << VBLANK_INTERRUPT_BIT) != 0 {
            (VBLANK_VECTOR, VBLANK_INTERRUPT_BIT)
        } else if pending & (1 << LCD_STAT_INTERRUPT_BIT) != 0 {
            (LCD_STAT_VECTOR, LCD_STAT_INTERRUPT_BIT)
        } else if pending & (1 << TIMER_INTERRUPT_BIT) != 0 {
            (TIMER_VECTOR, TIMER_INTERRUPT_BIT)
        } else if pending & (1 << SERIAL_INTERRUPT_BIT) != 0 {
            (SERIAL_VECTOR, SERIAL_INTERRUPT_BIT)
        } else if pending & (1 << JOYPAD_INTERRUPT_BIT) != 0 {
            (JOYPAD_VECTOR, JOYPAD_INTERRUPT_BIT)
        } else {
            unreachable!();
        };

        let current_if = memory_bus.read_byte(memory_map::IF_ADDR);
        memory_bus.write_byte(memory_map::IF_ADDR, current_if & !(1 << interrupt_bit));
        self.push_word(self.pc, memory_bus);
        self.pc = vector;
        20 // Interrupt handling cycles
    }

    // --- Memory Access Helpers ---
    #[inline(always)]
    fn read_byte_at_pc(&self, memory_bus: &MemoryBus) -> u8 {
        memory_bus.read_byte(self.pc)
    }
    #[inline(always)]
    fn read_d8(&self, memory_bus: &MemoryBus) -> u8 {
        memory_bus.read_byte(self.instruction_pc.wrapping_add(1))
    }
    #[inline(always)]
    fn read_d16(&self, memory_bus: &MemoryBus) -> u16 {
        let lo = memory_bus.read_byte(self.instruction_pc.wrapping_add(1));
        let hi = memory_bus.read_byte(self.instruction_pc.wrapping_add(2));
        u16::from_le_bytes([lo, hi])
    }
    #[inline(always)]
    fn read_r8(&self, memory_bus: &MemoryBus) -> i8 {
        memory_bus.read_byte(self.instruction_pc.wrapping_add(1)) as i8
    }

    // --- Stack Operations ---
    #[inline(always)]
    fn push_word(&mut self, value: u16, memory_bus: &mut MemoryBus) {
        self.sp = self.sp.wrapping_sub(1);
        memory_bus.write_byte(self.sp, (value >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        memory_bus.write_byte(self.sp, (value & 0xFF) as u8);
    }
    #[inline(always)]
    fn pop_word(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let low = memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (high << 8) | low
    }

    // --- Flag/Register Pair Helpers ---
    #[inline(always)]
    fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }
    #[inline(always)]
    fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0x00F0) as u8;
    }
    #[inline(always)]
    fn get_bc(&self) -> u16 {
        u16::from_le_bytes([self.c, self.b])
    }
    #[inline(always)]
    fn set_bc(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.c = bytes[0];
        self.b = bytes[1];
    }
    #[inline(always)]
    fn get_de(&self) -> u16 {
        u16::from_le_bytes([self.e, self.d])
    }
    #[inline(always)]
    fn set_de(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.e = bytes[0];
        self.d = bytes[1];
    }
    #[inline(always)]
    fn get_hl(&self) -> u16 {
        u16::from_le_bytes([self.l, self.h])
    }
    #[inline(always)]
    fn set_hl(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.l = bytes[0];
        self.h = bytes[1];
    }
    #[inline(always)]
    fn set_flag(&mut self, flag_mask: u8, set: bool) {
        if set {
            self.f |= flag_mask;
        } else {
            self.f &= !flag_mask;
        }
        self.f &= 0xF0;
    }
    #[inline(always)]
    fn get_flag(&self, flag_mask: u8) -> bool {
        (self.f & flag_mask) != 0
    }

    // --- ALU and Bit Operation Helpers ---
    // (Keep these internal helpers within the main impl block)
    fn inc_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, (value & 0x0F) == 0x0F);
        result
    }
    fn dec_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, (value & 0x0F) == 0x00);
        result
    }
    fn add_hl(&mut self, value: u16) {
        let hl = self.get_hl();
        let (result, carry) = hl.overflowing_add(value);
        let half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
        self.set_hl(result);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
    }
    fn add_a(&mut self, value: u8, use_carry: bool) {
        let carry_in = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };
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
        let carry_in = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };
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
    fn and_a(&mut self, value: u8, _use_carry: bool) {
        self.a &= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true);
        self.set_flag(FLAG_C, false);
    }
    fn xor_a(&mut self, value: u8, _use_carry: bool) {
        self.a ^= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
    }
    fn or_a(&mut self, value: u8, _use_carry: bool) {
        self.a |= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
    }
    fn cp_a(&mut self, value: u8, _use_carry: bool) {
        let temp_a = self.a;
        self.sub_a(value, false);
        self.a = temp_a;
    }
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
        let result = (value >> 1) | (value & 0x80);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }
    fn swap(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(4);
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
    }
    fn daa(&mut self) {
        let mut adjustment = 0u8;
        let mut set_carry = false;
        let n_flag = self.get_flag(FLAG_N);
        let h_flag = self.get_flag(FLAG_H);
        let c_flag = self.get_flag(FLAG_C);
        if !n_flag {
            if c_flag || self.a > 0x99 {
                adjustment |= 0x60;
                set_carry = true;
            }
            if h_flag || (self.a & 0x0F) > 0x09 {
                adjustment |= 0x06;
            }
            self.a = self.a.wrapping_add(adjustment);
        } else {
            if c_flag {
                adjustment |= 0x60;
                set_carry = true;
            }
            if h_flag {
                adjustment |= 0x06;
            }
            self.a = self.a.wrapping_sub(adjustment);
        }
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, set_carry);
    }

    // --- Public accessors ---
    #[inline(always)]
    pub fn pc(&self) -> u16 {
        self.pc
    }
    #[inline(always)]
    pub fn sp(&self) -> u16 {
        self.sp
    }
    #[inline(always)]
    pub fn registers(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8) {
        (
            self.a, self.f, self.b, self.c, self.d, self.e, self.h, self.l,
        )
    }
    #[inline(always)]
    pub fn ime(&self) -> bool {
        self.ime
    }
    #[inline(always)]
    pub fn halted(&self) -> bool {
        self.halted
    }
    #[inline(always)]
    pub fn stopped(&self) -> bool {
        self.stop_requested
    }
    #[inline(always)]
    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    // --- Debugging Helpers ---
    pub fn disassemble_instruction(&self, address: u16, bus: &MemoryBus) -> (String, u8) {
        let opcode = bus.read_byte(address);
        if opcode == 0xCB {
            let cb_opcode = bus.read_byte(address.wrapping_add(1));
            if let Some(cb_instr) = CB_INSTRUCTIONS.get(cb_opcode as usize) {
                (format!("{}", cb_instr.mnemonic), 2)
            } else {
                (format!("DB CB, {:02X}", cb_opcode), 2)
            }
        } else {
            if let Some(instr) = INSTRUCTIONS.get(opcode as usize) {
                let operand_str = match instr.length {
                    1 => "".to_string(),
                    2 => {
                        let d8 = bus.read_byte(address.wrapping_add(1));
                        if instr.mnemonic.starts_with("JR")
                            || instr.mnemonic == "ADD SP, r8"
                            || instr.mnemonic == "LD HL, SP+r8"
                        {
                            format!(" ${:+}", d8 as i8)
                        } else {
                            format!(" ${:02X}", d8)
                        }
                    }
                    3 => {
                        let lo = bus.read_byte(address.wrapping_add(1));
                        let hi = bus.read_byte(address.wrapping_add(2));
                        format!(" ${:04X}", u16::from_le_bytes([lo, hi]))
                    }
                    _ => "".to_string(),
                };
                let formatted_mnemonic = instr
                    .mnemonic
                    .replace("d16", &operand_str)
                    .replace("a16", &operand_str)
                    .replace("d8", &operand_str)
                    .replace("r8", &operand_str)
                    .trim_end()
                    .to_string();
                (formatted_mnemonic, instr.length)
            } else {
                (format!("DB {:02X}", opcode), 1)
            }
        }
    }
}
