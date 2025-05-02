use crate::instruction::{CB_INSTRUCTIONS, INSTRUCTIONS}; // Import only what's needed
use crate::memory_bus::MemoryBus;
use log;

// --- Constants ---
// (Making these public might be needed if other modules use them directly)
pub const FLAG_Z_POS: u8 = 7;
pub const FLAG_N_POS: u8 = 6;
pub const FLAG_H_POS: u8 = 5;
pub const FLAG_C_POS: u8 = 4;

pub const FLAG_Z: u8 = 1 << FLAG_Z_POS;
pub const FLAG_N: u8 = 1 << FLAG_N_POS;
pub const FLAG_H: u8 = 1 << FLAG_H_POS;
pub const FLAG_C: u8 = 1 << FLAG_C_POS;

pub const VBLANK_VECTOR: u16 = 0x0040;
pub const LCD_STAT_VECTOR: u16 = 0x0048;
pub const TIMER_VECTOR: u16 = 0x0050;
pub const SERIAL_VECTOR: u16 = 0x0058;
pub const JOYPAD_VECTOR: u16 = 0x0060;

pub const IE_REGISTER: u16 = 0xFFFF;
pub const IF_REGISTER: u16 = 0xFF0F;

// --- Type Aliases ---
pub type CpuResult<T> = Result<T, String>;

// --- LD Macros ---
macro_rules! ld_r_r {
    // Register to Register
    ($name:ident, $r1:ident, $r2:ident) => {
        pub fn $name(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
            self.$r1 = self.$r2;
            Ok(0)
        }
    };
}
macro_rules! ld_r_hlp {
    // Load from (HL) into Register
    ($name:ident, $r1:ident) => {
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            self.$r1 = bus.read_byte(self.get_hl());
            Ok(0)
        }
    };
}
macro_rules! ld_hlp_r {
    // Load from Register into (HL)
    ($name:ident, $r2:ident) => {
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            bus.write_byte(self.get_hl(), self.$r2);
            Ok(0)
        }
    };
}

// --- ALU Macros ---
macro_rules! alu_a_r {
    // ALU A, Register
    ($name:ident, $op:ident, $r2:ident) => {
        // No carry
        pub fn $name(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
            self.$op(self.$r2, false);
            Ok(0)
        }
    };
    ($name:ident, $op:ident, $r2:ident, carry) => {
        // With carry
        pub fn $name(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
            self.$op(self.$r2, true);
            Ok(0)
        }
    };
}
macro_rules! alu_a_hlp {
    // ALU A, (HL)
    ($name:ident, $op:ident) => {
        // No carry
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            let addr = self.get_hl();
            let val = bus.read_byte(addr);
            self.$op(val, false);
            Ok(0)
        }
    };
    ($name:ident, $op:ident, carry) => {
        // With carry
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            let addr = self.get_hl();
            let val = bus.read_byte(addr);
            self.$op(val, true);
            Ok(0)
        }
    };
}

// --- CB Prefix Macros ---
macro_rules! cb_reg_op {
    // CB on Register
    ($name:ident, $op:ident, $reg:ident) => {
        pub fn $name(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
            self.$reg = self.$op(self.$reg);
            Ok(0)
        }
    };
    ($name:ident, bit, $bit:expr, $reg:ident) => {
        pub fn $name(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
            self.op_bit($bit, self.$reg);
            Ok(0)
        }
    };
    ($name:ident, res, $bit:expr, $reg:ident) => {
        pub fn $name(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
            self.$reg &= !(1 << $bit);
            Ok(0)
        }
    };
    ($name:ident, set, $bit:expr, $reg:ident) => {
        pub fn $name(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
            self.$reg |= (1 << $bit);
            Ok(0)
        }
    };
}
macro_rules! cb_hlp_op {
    // CB on (HL)
    ($name:ident, $op:ident) => {
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            let addr = self.get_hl();
            let value = bus.read_byte(addr);
            let result = self.$op(value);
            bus.write_byte(addr, result);
            Ok(0)
        }
    };
    ($name:ident, bit, $bit:expr) => {
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            let value = bus.read_byte(self.get_hl());
            self.op_bit($bit, value);
            Ok(0)
        }
    };
    ($name:ident, res, $bit:expr) => {
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            let addr = self.get_hl();
            let value = bus.read_byte(addr);
            let result = value & !(1 << $bit);
            bus.write_byte(addr, result);
            Ok(0)
        }
    };
    ($name:ident, set, $bit:expr) => {
        pub fn $name(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
            let addr = self.get_hl();
            let value = bus.read_byte(addr);
            let result = value | (1 << $bit);
            bus.write_byte(addr, result);
            Ok(0)
        }
    };
}

// --- CPU Struct Definition ---
#[derive(Debug, Clone)]
pub struct Cpu {
    // --- Fields (All public as requested) ---
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
    pub halted: bool,
    pub stop_requested: bool,
    pub ime_scheduled: bool,
    // Keep these private unless needed externally
    total_cycles: u64,
    fetched_opcode: u8,
    instruction_pc: u16,
}

// --- CPU Implementation ---
impl Cpu {
    pub fn new(skip_boot_rom: bool) -> Self {
        let (init_a, init_f, init_bc, init_de, init_hl, init_pc) = if skip_boot_rom {
            (0x01, 0xB0, 0x0013, 0x00D8, 0x014D, 0x0100)
        } else {
            (0x00, 0x00, 0x0000, 0x0000, 0x0000, 0x0000)
        };
        Cpu {
            a: init_a,
            f: init_f,
            b: (init_bc >> 8) as u8,
            c: init_bc as u8,
            d: (init_de >> 8) as u8,
            e: init_de as u8,
            h: (init_hl >> 8) as u8,
            l: init_hl as u8,
            sp: 0xFFFE,
            pc: init_pc,
            ime: false,
            halted: false,
            stop_requested: false,
            ime_scheduled: false,
            total_cycles: 0,
            fetched_opcode: 0,
            instruction_pc: 0,
        }
    }

    pub fn initialize_post_boot_io(memory_bus: &mut MemoryBus) {
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
        memory_bus.write_byte(0xFF26, 0xF1); // NR52 - F1 for DMG
        memory_bus.write_byte(0xFF40, 0x91); // LCDC
        memory_bus.write_byte(0xFF41, 0x85); // STAT
        memory_bus.write_byte(0xFF42, 0x00); // SCY
        memory_bus.write_byte(0xFF43, 0x00); // SCX
        memory_bus.write_byte(0xFF44, 0x00); // LY
        memory_bus.write_byte(0xFF45, 0x00); // LYC
        memory_bus.write_byte(0xFF47, 0xFC); // BGP
        memory_bus.write_byte(0xFF48, 0xFF); // OBP0
        memory_bus.write_byte(0xFF49, 0xFF); // OBP1
        memory_bus.write_byte(0xFF4A, 0x00); // WY
        memory_bus.write_byte(0xFF4B, 0x00); // WX
        memory_bus.write_byte(IE_REGISTER, 0x00); // IE
        memory_bus.write_byte(IF_REGISTER, 0x00); // IF
        memory_bus.write_byte(0xFF50, 0x01); // Boot ROM status
    }

    pub fn step(&mut self, memory_bus: &mut MemoryBus) -> CpuResult<u16> {
        // Handle pending EI
        let mut ime_just_enabled = false;
        if self.ime_scheduled {
            self.ime = true;
            self.ime_scheduled = false;
            ime_just_enabled = true;
        }

        // Handle Interrupts
        let interrupt_cycles = if self.ime && !ime_just_enabled {
            self.handle_interrupts(memory_bus)
        } else {
            0
        };
        if interrupt_cycles > 0 {
            self.halted = false;
            self.stop_requested = false;
            self.total_cycles += interrupt_cycles as u64;
            return Ok(interrupt_cycles as u16);
        }

        // Handle HALT/STOP
        if self.halted || self.stop_requested {
            if self.halted {
                let ie = memory_bus.read_byte(IE_REGISTER);
                let iflags = memory_bus.read_byte(IF_REGISTER);
                if (ie & iflags & 0x1F) != 0 {
                    self.halted = false;
                }
            }
            if self.halted || self.stop_requested {
                self.total_cycles += 4;
                return Ok(4);
            }
        }

        // Fetch
        self.instruction_pc = self.pc;
        self.fetched_opcode = self.read_byte_at_pc(memory_bus);

        // Decode
        let instruction = &INSTRUCTIONS[self.fetched_opcode as usize];

        // Advance PC
        self.pc = self.pc.wrapping_add(instruction.length as u16);

        // Execute
        let execute_result = (instruction.execute)(self, memory_bus);

        // Process Result
        match execute_result {
            Ok(additional_cycles) => {
                let total_instruction_cycles = instruction.cycles as u16 + additional_cycles;
                self.total_cycles += total_instruction_cycles as u64;
                Ok(total_instruction_cycles)
            }
            Err(error_message) => {
                log::error!(
                    "CPU Error at {:#06X} (Opcode {:#04X}): {}",
                    self.instruction_pc,
                    self.fetched_opcode,
                    error_message
                );
                self.total_cycles += instruction.cycles as u64; // Consume base cycles on error
                Err(format!(
                    "CPU Error at {:#06X} (Opcode {:#04X}): {}",
                    self.instruction_pc, self.fetched_opcode, error_message
                ))
            }
        }
    }

    pub fn handle_interrupts(&mut self, memory_bus: &mut MemoryBus) -> u8 {
        let if_flags = memory_bus.read_byte(IF_REGISTER);
        let ie_flags = memory_bus.read_byte(IE_REGISTER);
        let pending = if_flags & ie_flags & 0x1F;
        if pending == 0 {
            return 0;
        }

        self.ime = false;
        self.ime_scheduled = false;

        let vector;
        let interrupt_bit;
        if pending & 0x01 != 0 {
            vector = VBLANK_VECTOR;
            interrupt_bit = 0;
        } else if pending & 0x02 != 0 {
            vector = LCD_STAT_VECTOR;
            interrupt_bit = 1;
        } else if pending & 0x04 != 0 {
            vector = TIMER_VECTOR;
            interrupt_bit = 2;
        } else if pending & 0x08 != 0 {
            vector = SERIAL_VECTOR;
            interrupt_bit = 3;
        } else if pending & 0x10 != 0 {
            vector = JOYPAD_VECTOR;
            interrupt_bit = 4;
        } else {
            unreachable!();
        }

        let current_if = memory_bus.read_byte(IF_REGISTER);
        memory_bus.write_byte(IF_REGISTER, current_if & !(1 << interrupt_bit));
        self.push_word(self.pc, memory_bus);
        self.pc = vector;
        20 // Interrupt handling cycles
    }

    // --- Memory Access Helpers ---
    #[inline(always)]
    pub fn read_byte_at_pc(&self, memory_bus: &MemoryBus) -> u8 {
        memory_bus.read_byte(self.pc)
    }
    #[inline(always)]
    pub fn read_d8(&self, memory_bus: &MemoryBus) -> u8 {
        memory_bus.read_byte(self.instruction_pc.wrapping_add(1))
    }
    #[inline(always)]
    pub fn read_d16(&self, memory_bus: &MemoryBus) -> u16 {
        u16::from_le_bytes([
            memory_bus.read_byte(self.instruction_pc.wrapping_add(1)),
            memory_bus.read_byte(self.instruction_pc.wrapping_add(2)),
        ])
    }
    #[inline(always)]
    pub fn read_r8(&self, memory_bus: &MemoryBus) -> i8 {
        memory_bus.read_byte(self.instruction_pc.wrapping_add(1)) as i8
    }

    // --- Stack Operations ---
    #[inline(always)]
    pub fn push_word(&mut self, value: u16, memory_bus: &mut MemoryBus) {
        self.sp = self.sp.wrapping_sub(1);
        memory_bus.write_byte(self.sp, (value >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        memory_bus.write_byte(self.sp, (value & 0xFF) as u8);
    }
    #[inline(always)]
    pub fn pop_word(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let low = memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = memory_bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (high << 8) | low
    }

    // --- Flag/Register Helpers ---
    #[inline(always)]
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16 & 0xF0)
    }
    #[inline(always)]
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0x00F0) as u8;
    }
    #[inline(always)]
    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }
    #[inline(always)]
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0x00FF) as u8;
    }
    #[inline(always)]
    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }
    #[inline(always)]
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0x00FF) as u8;
    }
    #[inline(always)]
    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }
    #[inline(always)]
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0x00FF) as u8;
    }
    #[inline(always)]
    pub fn set_flag(&mut self, flag_mask: u8, set: bool) {
        if set {
            self.f |= flag_mask;
        } else {
            self.f &= !flag_mask;
        }
        self.f &= 0xF0;
    }
    #[inline(always)]
    pub fn get_flag(&self, flag_mask: u8) -> bool {
        (self.f & flag_mask) != 0
    }

    // --- ALU and Bit Operation Helpers (All public) ---
    pub fn inc_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, (value & 0x0F) == 0x0F);
        result
    }
    pub fn dec_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, (value & 0x0F) == 0x00);
        result
    }
    pub fn add_hl(&mut self, value: u16) {
        let hl = self.get_hl();
        let (result, carry) = hl.overflowing_add(value);
        let half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
        self.set_hl(result);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
    }
    pub fn add_a(&mut self, value: u8, use_carry: bool) {
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
    pub fn sub_a(&mut self, value: u8, use_carry: bool) {
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
    pub fn and_a(&mut self, value: u8, _use_carry: bool) {
        self.a &= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true);
        self.set_flag(FLAG_C, false);
    }
    pub fn xor_a(&mut self, value: u8, _use_carry: bool) {
        self.a ^= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
    }
    pub fn or_a(&mut self, value: u8, _use_carry: bool) {
        self.a |= value;
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
    }
    pub fn cp_a(&mut self, value: u8, _use_carry: bool) {
        let temp_a = self.a;
        self.sub_a(value, false);
        self.a = temp_a;
    }
    pub fn rlc(&mut self, value: u8) -> u8 {
        let carry = (value >> 7) & 1;
        let result = value.rotate_left(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }
    pub fn rrc(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let result = value.rotate_right(1);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }
    pub fn rl(&mut self, value: u8) -> u8 {
        let old_carry = self.get_flag(FLAG_C) as u8;
        let new_carry = (value >> 7) & 1;
        let result = (value << 1) | old_carry;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, new_carry != 0);
        result
    }
    pub fn rr(&mut self, value: u8) -> u8 {
        let old_carry = self.get_flag(FLAG_C) as u8;
        let new_carry = value & 1;
        let result = (value >> 1) | (old_carry << 7);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, new_carry != 0);
        result
    }
    pub fn sla(&mut self, value: u8) -> u8 {
        let carry = (value >> 7) & 1;
        let result = value << 1;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }
    pub fn sra(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let result = (value >> 1) | (value & 0x80);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }
    pub fn swap(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(4);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H | FLAG_C, false);
        result
    }
    pub fn srl(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let result = value >> 1;
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, carry != 0);
        result
    }
    pub fn op_bit(&mut self, bit: u8, value: u8) {
        let result_zero = (value >> bit) & 1 == 0;
        self.set_flag(FLAG_Z, result_zero);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true);
    }
    pub fn daa(&mut self) {
        let mut adjustment = 0u8;
        let mut set_carry = false;
        let original_a = self.a;
        let original_c = self.get_flag(FLAG_C);
        let original_h = self.get_flag(FLAG_H);
        let n_flag = self.get_flag(FLAG_N);
        if !n_flag {
            if original_c || original_a > 0x99 {
                adjustment |= 0x60;
                set_carry = true;
            }
            if original_h || (original_a & 0x0F) > 9 {
                adjustment |= 0x06;
            }
            self.a = self.a.wrapping_add(adjustment);
        } else {
            if original_c {
                adjustment |= 0x60;
                set_carry = true;
            }
            if original_h {
                adjustment |= 0x06;
            }
            self.a = self.a.wrapping_sub(adjustment);
        }
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, set_carry);
    }

    // --- Invalid Opcode Handler ---
    pub fn handle_invalid_opcode(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        Err(format!("Invalid/Unknown Opcode"))
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
            self.a,
            self.f & 0xF0,
            self.b,
            self.c,
            self.d,
            self.e,
            self.h,
            self.l,
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
    // Expose total_cycles if needed externally
    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

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

    // --- Individual Instruction Implementations (All public) ---
    pub fn op_nop(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        Ok(0)
    }
    pub fn op_ld_bc_d16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_bc(self.read_d16(bus));
        Ok(0)
    }
    pub fn op_ld_bc_a(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        bus.write_byte(self.get_bc(), self.a);
        Ok(0)
    }
    pub fn op_inc_bc(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_bc(self.get_bc().wrapping_add(1));
        Ok(0)
    }
    pub fn op_inc_b(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.b = self.inc_u8(self.b);
        Ok(0)
    }
    pub fn op_dec_b(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.b = self.dec_u8(self.b);
        Ok(0)
    }
    pub fn op_ld_b_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.b = self.read_d8(bus);
        Ok(0)
    }
    pub fn op_rlca(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rlc(self.a);
        self.set_flag(FLAG_Z, false);
        Ok(0)
    }
    pub fn op_ld_a16_sp(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        bus.write_byte(addr, (self.sp & 0xFF) as u8);
        bus.write_byte(addr.wrapping_add(1), (self.sp >> 8) as u8);
        Ok(0)
    }
    pub fn op_add_hl_bc(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.get_bc());
        Ok(0)
    }
    pub fn op_ld_a_bc(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = bus.read_byte(self.get_bc());
        Ok(0)
    }
    pub fn op_dec_bc(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_bc(self.get_bc().wrapping_sub(1));
        Ok(0)
    }
    pub fn op_inc_c(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.c = self.inc_u8(self.c);
        Ok(0)
    }
    pub fn op_dec_c(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.c = self.dec_u8(self.c);
        Ok(0)
    }
    pub fn op_ld_c_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.c = self.read_d8(bus);
        Ok(0)
    }
    pub fn op_rrca(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rrc(self.a);
        self.set_flag(FLAG_Z, false);
        Ok(0)
    }
    pub fn op_stop(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.stop_requested = true;
        log::warn!(
            "STOP instruction encountered at PC={:#06X}",
            self.instruction_pc
        );
        Ok(0)
    }
    pub fn op_ld_de_d16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_de(self.read_d16(bus));
        Ok(0)
    }
    pub fn op_ld_de_a(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        bus.write_byte(self.get_de(), self.a);
        Ok(0)
    }
    pub fn op_inc_de(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_de(self.get_de().wrapping_add(1));
        Ok(0)
    }
    pub fn op_inc_d(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.d = self.inc_u8(self.d);
        Ok(0)
    }
    pub fn op_dec_d(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.d = self.dec_u8(self.d);
        Ok(0)
    }
    pub fn op_ld_d_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.d = self.read_d8(bus);
        Ok(0)
    }
    pub fn op_rla(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rl(self.a);
        self.set_flag(FLAG_Z, false);
        Ok(0)
    }
    pub fn op_jr_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        self.pc = self.pc.wrapping_add(offset as i16 as u16);
        Ok(0)
    }
    pub fn op_add_hl_de(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.get_de());
        Ok(0)
    }
    pub fn op_ld_a_de(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = bus.read_byte(self.get_de());
        Ok(0)
    }
    pub fn op_dec_de(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_de(self.get_de().wrapping_sub(1));
        Ok(0)
    }
    pub fn op_inc_e(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.e = self.inc_u8(self.e);
        Ok(0)
    }
    pub fn op_dec_e(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.e = self.dec_u8(self.e);
        Ok(0)
    }
    pub fn op_ld_e_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.e = self.read_d8(bus);
        Ok(0)
    }
    pub fn op_rra(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rr(self.a);
        self.set_flag(FLAG_Z, false);
        Ok(0)
    }
    pub fn op_jr_nz_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        if !self.get_flag(FLAG_Z) {
            self.pc = self.pc.wrapping_add(offset as i16 as u16);
            Ok(4)
        } else {
            Ok(0)
        }
    }
    pub fn op_ld_hl_d16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_hl(self.read_d16(bus));
        Ok(0)
    }
    pub fn op_ld_hli_a(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        bus.write_byte(addr, self.a);
        self.set_hl(addr.wrapping_add(1));
        Ok(0)
    }
    pub fn op_inc_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_hl(self.get_hl().wrapping_add(1));
        Ok(0)
    }
    pub fn op_inc_h(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.h = self.inc_u8(self.h);
        Ok(0)
    }
    pub fn op_dec_h(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.h = self.dec_u8(self.h);
        Ok(0)
    }
    pub fn op_ld_h_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.h = self.read_d8(bus);
        Ok(0)
    }
    pub fn op_daa(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.daa();
        Ok(0)
    }
    pub fn op_jr_z_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        if self.get_flag(FLAG_Z) {
            self.pc = self.pc.wrapping_add(offset as i16 as u16);
            Ok(4)
        } else {
            Ok(0)
        }
    }
    pub fn op_add_hl_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.get_hl());
        Ok(0)
    }
    pub fn op_ld_a_hli(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        self.a = bus.read_byte(addr);
        self.set_hl(addr.wrapping_add(1));
        Ok(0)
    }
    pub fn op_dec_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_hl(self.get_hl().wrapping_sub(1));
        Ok(0)
    }
    pub fn op_inc_l(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.l = self.inc_u8(self.l);
        Ok(0)
    }
    pub fn op_dec_l(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.l = self.dec_u8(self.l);
        Ok(0)
    }
    pub fn op_ld_l_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.l = self.read_d8(bus);
        Ok(0)
    }
    pub fn op_cpl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = !self.a;
        self.set_flag(FLAG_N | FLAG_H, true);
        Ok(0)
    }
    pub fn op_jr_nc_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        if !self.get_flag(FLAG_C) {
            self.pc = self.pc.wrapping_add(offset as i16 as u16);
            Ok(4)
        } else {
            Ok(0)
        }
    }
    pub fn op_ld_sp_d16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sp = self.read_d16(bus);
        Ok(0)
    }
    pub fn op_ld_hld_a(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        bus.write_byte(addr, self.a);
        self.set_hl(addr.wrapping_sub(1));
        Ok(0)
    }
    pub fn op_inc_sp(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sp = self.sp.wrapping_add(1);
        Ok(0)
    }
    pub fn op_inc_hlp(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        let value = bus.read_byte(addr);
        let result = self.inc_u8(value);
        bus.write_byte(addr, result);
        Ok(0)
    }
    pub fn op_dec_hlp(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        let value = bus.read_byte(addr);
        let result = self.dec_u8(value);
        bus.write_byte(addr, result);
        Ok(0)
    }
    pub fn op_ld_hlp_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let value = self.read_d8(bus);
        bus.write_byte(self.get_hl(), value);
        Ok(0)
    }
    pub fn op_scf(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, true);
        Ok(0)
    }
    pub fn op_jr_c_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        if self.get_flag(FLAG_C) {
            self.pc = self.pc.wrapping_add(offset as i16 as u16);
            Ok(4)
        } else {
            Ok(0)
        }
    }
    pub fn op_add_hl_sp(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.sp);
        Ok(0)
    }
    pub fn op_ld_a_hld(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        self.a = bus.read_byte(addr);
        self.set_hl(addr.wrapping_sub(1));
        Ok(0)
    }
    pub fn op_dec_sp(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sp = self.sp.wrapping_sub(1);
        Ok(0)
    }
    pub fn op_inc_a(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.inc_u8(self.a);
        Ok(0)
    }
    pub fn op_dec_a(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.dec_u8(self.a);
        Ok(0)
    }
    pub fn op_ld_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.read_d8(bus);
        Ok(0)
    }
    pub fn op_ccf(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        let current_c = self.get_flag(FLAG_C);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, !current_c);
        Ok(0)
    }

    // --- LD r, r' / HALT (0x40 - 0x7F) ---
    // Uses ld_r_r!, ld_r_hlp!, ld_hlp_r! macros
    ld_r_r!(op_ld_b_b, b, b);
    ld_r_r!(op_ld_b_c, b, c);
    ld_r_r!(op_ld_b_d, b, d);
    ld_r_r!(op_ld_b_e, b, e);
    ld_r_r!(op_ld_b_h, b, h);
    ld_r_r!(op_ld_b_l, b, l);
    ld_r_hlp!(op_ld_b_hlp, b);
    ld_r_r!(op_ld_b_a, b, a);
    ld_r_r!(op_ld_c_b, c, b);
    ld_r_r!(op_ld_c_c, c, c);
    ld_r_r!(op_ld_c_d, c, d);
    ld_r_r!(op_ld_c_e, c, e);
    ld_r_r!(op_ld_c_h, c, h);
    ld_r_r!(op_ld_c_l, c, l);
    ld_r_hlp!(op_ld_c_hlp, c);
    ld_r_r!(op_ld_c_a, c, a);
    ld_r_r!(op_ld_d_b, d, b);
    ld_r_r!(op_ld_d_c, d, c);
    ld_r_r!(op_ld_d_d, d, d);
    ld_r_r!(op_ld_d_e, d, e);
    ld_r_r!(op_ld_d_h, d, h);
    ld_r_r!(op_ld_d_l, d, l);
    ld_r_hlp!(op_ld_d_hlp, d);
    ld_r_r!(op_ld_d_a, d, a);
    ld_r_r!(op_ld_e_b, e, b);
    ld_r_r!(op_ld_e_c, e, c);
    ld_r_r!(op_ld_e_d, e, d);
    ld_r_r!(op_ld_e_e, e, e);
    ld_r_r!(op_ld_e_h, e, h);
    ld_r_r!(op_ld_e_l, e, l);
    ld_r_hlp!(op_ld_e_hlp, e);
    ld_r_r!(op_ld_e_a, e, a);
    ld_r_r!(op_ld_h_b, h, b);
    ld_r_r!(op_ld_h_c, h, c);
    ld_r_r!(op_ld_h_d, h, d);
    ld_r_r!(op_ld_h_e, h, e);
    ld_r_r!(op_ld_h_h, h, h);
    ld_r_r!(op_ld_h_l, h, l);
    ld_r_hlp!(op_ld_h_hlp, h);
    ld_r_r!(op_ld_h_a, h, a);
    ld_r_r!(op_ld_l_b, l, b);
    ld_r_r!(op_ld_l_c, l, c);
    ld_r_r!(op_ld_l_d, l, d);
    ld_r_r!(op_ld_l_e, l, e);
    ld_r_r!(op_ld_l_h, l, h);
    ld_r_r!(op_ld_l_l, l, l);
    ld_r_hlp!(op_ld_l_hlp, l);
    ld_r_r!(op_ld_l_a, l, a);
    ld_hlp_r!(op_ld_hlp_b, b);
    ld_hlp_r!(op_ld_hlp_c, c);
    ld_hlp_r!(op_ld_hlp_d, d);
    ld_hlp_r!(op_ld_hlp_e, e);
    ld_hlp_r!(op_ld_hlp_h, h);
    ld_hlp_r!(op_ld_hlp_l, l);
    ld_hlp_r!(op_ld_hlp_a, a);
    ld_r_r!(op_ld_a_b, a, b);
    ld_r_r!(op_ld_a_c, a, c);
    ld_r_r!(op_ld_a_d, a, d);
    ld_r_r!(op_ld_a_e, a, e);
    ld_r_r!(op_ld_a_h, a, h);
    ld_r_r!(op_ld_a_l, a, l);
    ld_r_hlp!(op_ld_a_hlp, a);
    ld_r_r!(op_ld_a_a, a, a);
    pub fn op_halt(&mut self, memory_bus: &mut MemoryBus) -> CpuResult<u16> {
        let ie = memory_bus.read_byte(IE_REGISTER);
        let iflags = memory_bus.read_byte(IF_REGISTER);
        if !self.ime && (ie & iflags & 0x1F) != 0 {
            log::warn!(
                "HALT bug triggered at PC={:#06X}! IME=0, IE&IF={:02X}",
                self.instruction_pc,
                ie & iflags
            );
        } else {
            self.halted = true;
        }
        Ok(0)
    }

    // --- ALU A, r / ALU A, (HL) (0x80 - 0xBF) ---
    // Uses alu_a_r!, alu_a_hlp! macros
    alu_a_r!(op_add_a_b, add_a, b);
    alu_a_r!(op_add_a_c, add_a, c);
    alu_a_r!(op_add_a_d, add_a, d);
    alu_a_r!(op_add_a_e, add_a, e);
    alu_a_r!(op_add_a_h, add_a, h);
    alu_a_r!(op_add_a_l, add_a, l);
    alu_a_hlp!(op_add_a_hlp, add_a);
    alu_a_r!(op_add_a_a, add_a, a);
    alu_a_r!(op_adc_a_b, add_a, b, carry);
    alu_a_r!(op_adc_a_c, add_a, c, carry);
    alu_a_r!(op_adc_a_d, add_a, d, carry);
    alu_a_r!(op_adc_a_e, add_a, e, carry);
    alu_a_r!(op_adc_a_h, add_a, h, carry);
    alu_a_r!(op_adc_a_l, add_a, l, carry);
    alu_a_hlp!(op_adc_a_hlp, add_a, carry);
    alu_a_r!(op_adc_a_a, add_a, a, carry);
    alu_a_r!(op_sub_a_b, sub_a, b);
    alu_a_r!(op_sub_a_c, sub_a, c);
    alu_a_r!(op_sub_a_d, sub_a, d);
    alu_a_r!(op_sub_a_e, sub_a, e);
    alu_a_r!(op_sub_a_h, sub_a, h);
    alu_a_r!(op_sub_a_l, sub_a, l);
    alu_a_hlp!(op_sub_a_hlp, sub_a);
    alu_a_r!(op_sub_a_a, sub_a, a);
    alu_a_r!(op_sbc_a_b, sub_a, b, carry);
    alu_a_r!(op_sbc_a_c, sub_a, c, carry);
    alu_a_r!(op_sbc_a_d, sub_a, d, carry);
    alu_a_r!(op_sbc_a_e, sub_a, e, carry);
    alu_a_r!(op_sbc_a_h, sub_a, h, carry);
    alu_a_r!(op_sbc_a_l, sub_a, l, carry);
    alu_a_hlp!(op_sbc_a_hlp, sub_a, carry);
    alu_a_r!(op_sbc_a_a, sub_a, a, carry);
    alu_a_r!(op_and_a_b, and_a, b);
    alu_a_r!(op_and_a_c, and_a, c);
    alu_a_r!(op_and_a_d, and_a, d);
    alu_a_r!(op_and_a_e, and_a, e);
    alu_a_r!(op_and_a_h, and_a, h);
    alu_a_r!(op_and_a_l, and_a, l);
    alu_a_hlp!(op_and_a_hlp, and_a);
    alu_a_r!(op_and_a_a, and_a, a);
    alu_a_r!(op_xor_a_b, xor_a, b);
    alu_a_r!(op_xor_a_c, xor_a, c);
    alu_a_r!(op_xor_a_d, xor_a, d);
    alu_a_r!(op_xor_a_e, xor_a, e);
    alu_a_r!(op_xor_a_h, xor_a, h);
    alu_a_r!(op_xor_a_l, xor_a, l);
    alu_a_hlp!(op_xor_a_hlp, xor_a);
    alu_a_r!(op_xor_a_a, xor_a, a);
    alu_a_r!(op_or_a_b, or_a, b);
    alu_a_r!(op_or_a_c, or_a, c);
    alu_a_r!(op_or_a_d, or_a, d);
    alu_a_r!(op_or_a_e, or_a, e);
    alu_a_r!(op_or_a_h, or_a, h);
    alu_a_r!(op_or_a_l, or_a, l);
    alu_a_hlp!(op_or_a_hlp, or_a);
    alu_a_r!(op_or_a_a, or_a, a);
    alu_a_r!(op_cp_a_b, cp_a, b);
    alu_a_r!(op_cp_a_c, cp_a, c);
    alu_a_r!(op_cp_a_d, cp_a, d);
    alu_a_r!(op_cp_a_e, cp_a, e);
    alu_a_r!(op_cp_a_h, cp_a, h);
    alu_a_r!(op_cp_a_l, cp_a, l);
    alu_a_hlp!(op_cp_a_hlp, cp_a);
    alu_a_r!(op_cp_a_a, cp_a, a);

    // --- Jumps, Calls, Returns, RST (0xC0 - 0xFF) ---
    pub fn conditional_ret(&mut self, condition: bool, bus: &mut MemoryBus) -> CpuResult<u16> {
        if condition {
            self.pc = self.pop_word(bus);
            Ok(12)
        } else {
            Ok(0)
        }
    }
    pub fn op_ret_nz(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(!self.get_flag(FLAG_Z), bus)
    }
    pub fn op_pop_bc(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let val = self.pop_word(bus);
        self.set_bc(val);
        Ok(0)
    }
    pub fn conditional_jp_a16(&mut self, condition: bool, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        if condition {
            self.pc = addr;
            Ok(4)
        } else {
            Ok(0)
        }
    }
    pub fn op_jp_nz_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(!self.get_flag(FLAG_Z), bus)
    }
    pub fn op_jp_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.read_d16(bus);
        Ok(0)
    }
    pub fn conditional_call_a16(&mut self, condition: bool, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        if condition {
            self.push_word(self.pc, bus);
            self.pc = addr;
            Ok(12)
        } else {
            Ok(0)
        }
    }
    pub fn op_call_nz_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(!self.get_flag(FLAG_Z), bus)
    }
    pub fn op_push_bc(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.push_word(self.get_bc(), bus);
        Ok(0)
    }
    pub fn op_add_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_a(self.read_d8(bus), false);
        Ok(0)
    }
    pub fn rst(&mut self, vector: u16, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.push_word(self.pc, bus);
        self.pc = vector;
        Ok(0)
    }
    pub fn op_rst_00h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0000, bus)
    }
    pub fn op_ret_z(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(self.get_flag(FLAG_Z), bus)
    }
    pub fn op_ret(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.pop_word(bus);
        Ok(0)
    }
    pub fn op_jp_z_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(self.get_flag(FLAG_Z), bus)
    }
    pub fn op_prefix_cb(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let cb_opcode = bus.read_byte(self.instruction_pc.wrapping_add(1));
        let cb_instr = &CB_INSTRUCTIONS[cb_opcode as usize];
        let cb_result = (cb_instr.execute)(self, bus);
        match cb_result {
            Ok(_) => Ok(cb_instr.cycles as u16),
            Err(e) => Err(format!("CB Opcode {:#04X} Error: {}", cb_opcode, e)),
        }
    }
    pub fn op_call_z_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(self.get_flag(FLAG_Z), bus)
    }
    pub fn op_call_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        self.push_word(self.pc, bus);
        self.pc = addr;
        Ok(0)
    }
    pub fn op_adc_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_a(self.read_d8(bus), true);
        Ok(0)
    }
    pub fn op_rst_08h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0008, bus)
    }
    pub fn op_ret_nc(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(!self.get_flag(FLAG_C), bus)
    }
    pub fn op_pop_de(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let val = self.pop_word(bus);
        self.set_de(val);
        Ok(0)
    }
    pub fn op_jp_nc_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(!self.get_flag(FLAG_C), bus)
    }
    pub fn op_call_nc_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(!self.get_flag(FLAG_C), bus)
    }
    pub fn op_push_de(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.push_word(self.get_de(), bus);
        Ok(0)
    }
    pub fn op_sub_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sub_a(self.read_d8(bus), false);
        Ok(0)
    }
    pub fn op_rst_10h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0010, bus)
    }
    pub fn op_ret_c(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(self.get_flag(FLAG_C), bus)
    }
    pub fn op_reti(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.pop_word(bus);
        self.ime = true;
        self.ime_scheduled = false;
        Ok(0)
    }
    pub fn op_jp_c_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(self.get_flag(FLAG_C), bus)
    }
    pub fn op_call_c_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(self.get_flag(FLAG_C), bus)
    }
    pub fn op_sbc_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sub_a(self.read_d8(bus), true);
        Ok(0)
    }
    pub fn op_rst_18h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0018, bus)
    }
    pub fn op_ldh_a8_a(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_d8(bus) as u16;
        bus.write_byte(0xFF00 + offset, self.a);
        Ok(0)
    }
    pub fn op_pop_hl(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let val = self.pop_word(bus);
        self.set_hl(val);
        Ok(0)
    }
    pub fn op_ld_cp_a(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        bus.write_byte(0xFF00 + self.c as u16, self.a);
        Ok(0)
    }
    pub fn op_push_hl(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.push_word(self.get_hl(), bus);
        Ok(0)
    }
    pub fn op_and_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.and_a(self.read_d8(bus), false);
        Ok(0)
    }
    pub fn op_rst_20h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0020, bus)
    }
    pub fn op_add_sp_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        let value = offset as i16 as u16;
        let sp = self.sp;
        let result = sp.wrapping_add(value);
        let half_carry = (sp & 0x000F) + (value & 0x000F) > 0x000F;
        let carry = (sp & 0x00FF) + (value & 0x00FF) > 0x00FF;
        self.sp = result;
        self.set_flag(FLAG_Z | FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
        Ok(0)
    }
    pub fn op_jp_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.get_hl();
        Ok(0)
    }
    pub fn op_ld_a16_a(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        bus.write_byte(addr, self.a);
        Ok(0)
    }
    pub fn op_xor_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.xor_a(self.read_d8(bus), false);
        Ok(0)
    }
    pub fn op_rst_28h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0028, bus)
    }
    pub fn op_ldh_a_a8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_d8(bus) as u16;
        self.a = bus.read_byte(0xFF00 + offset);
        Ok(0)
    }
    pub fn op_pop_af(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let val = self.pop_word(bus);
        self.set_af(val);
        Ok(0)
    }
    pub fn op_ld_a_cp(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = bus.read_byte(0xFF00 + self.c as u16);
        Ok(0)
    }
    pub fn op_di(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.ime = false;
        self.ime_scheduled = false;
        Ok(0)
    }
    pub fn op_push_af(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.f &= 0xF0;
        self.push_word(self.get_af(), bus);
        Ok(0)
    }
    pub fn op_or_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.or_a(self.read_d8(bus), false);
        Ok(0)
    }
    pub fn op_rst_30h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0030, bus)
    }
    pub fn op_ld_hl_sp_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        let value = offset as i16 as u16;
        let sp = self.sp;
        let result = sp.wrapping_add(value);
        let half_carry = (sp & 0x000F) + (value & 0x000F) > 0x000F;
        let carry = (sp & 0x00FF) + (value & 0x00FF) > 0x00FF;
        self.set_hl(result);
        self.set_flag(FLAG_Z | FLAG_N, false);
        self.set_flag(FLAG_H, half_carry);
        self.set_flag(FLAG_C, carry);
        Ok(0)
    }
    pub fn op_ld_sp_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sp = self.get_hl();
        Ok(0)
    }
    pub fn op_ld_a_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        self.a = bus.read_byte(addr);
        Ok(0)
    }
    pub fn op_ei(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.ime_scheduled = true;
        Ok(0)
    }
    pub fn op_cp_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.cp_a(self.read_d8(bus), false);
        Ok(0)
    }
    pub fn op_rst_38h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0038, bus)
    }

    // --- CB Prefix Implementations (uses cb_reg_op!, cb_hlp_op!) ---
    // RLC r / RLC (HL)
    cb_reg_op!(cb_rlc_b, rlc, b);
    cb_reg_op!(cb_rlc_c, rlc, c);
    cb_reg_op!(cb_rlc_d, rlc, d);
    cb_reg_op!(cb_rlc_e, rlc, e);
    cb_reg_op!(cb_rlc_h, rlc, h);
    cb_reg_op!(cb_rlc_l, rlc, l);
    cb_hlp_op!(cb_rlc_hlp, rlc);
    cb_reg_op!(cb_rlc_a, rlc, a);
    // RRC r / RRC (HL)
    cb_reg_op!(cb_rrc_b, rrc, b);
    cb_reg_op!(cb_rrc_c, rrc, c);
    cb_reg_op!(cb_rrc_d, rrc, d);
    cb_reg_op!(cb_rrc_e, rrc, e);
    cb_reg_op!(cb_rrc_h, rrc, h);
    cb_reg_op!(cb_rrc_l, rrc, l);
    cb_hlp_op!(cb_rrc_hlp, rrc);
    cb_reg_op!(cb_rrc_a, rrc, a);
    // RL r / RL (HL)
    cb_reg_op!(cb_rl_b, rl, b);
    cb_reg_op!(cb_rl_c, rl, c);
    cb_reg_op!(cb_rl_d, rl, d);
    cb_reg_op!(cb_rl_e, rl, e);
    cb_reg_op!(cb_rl_h, rl, h);
    cb_reg_op!(cb_rl_l, rl, l);
    cb_hlp_op!(cb_rl_hlp, rl);
    cb_reg_op!(cb_rl_a, rl, a);
    // RR r / RR (HL)
    cb_reg_op!(cb_rr_b, rr, b);
    cb_reg_op!(cb_rr_c, rr, c);
    cb_reg_op!(cb_rr_d, rr, d);
    cb_reg_op!(cb_rr_e, rr, e);
    cb_reg_op!(cb_rr_h, rr, h);
    cb_reg_op!(cb_rr_l, rr, l);
    cb_hlp_op!(cb_rr_hlp, rr);
    cb_reg_op!(cb_rr_a, rr, a);
    // SLA r / SLA (HL)
    cb_reg_op!(cb_sla_b, sla, b);
    cb_reg_op!(cb_sla_c, sla, c);
    cb_reg_op!(cb_sla_d, sla, d);
    cb_reg_op!(cb_sla_e, sla, e);
    cb_reg_op!(cb_sla_h, sla, h);
    cb_reg_op!(cb_sla_l, sla, l);
    cb_hlp_op!(cb_sla_hlp, sla);
    cb_reg_op!(cb_sla_a, sla, a);
    // SRA r / SRA (HL)
    cb_reg_op!(cb_sra_b, sra, b);
    cb_reg_op!(cb_sra_c, sra, c);
    cb_reg_op!(cb_sra_d, sra, d);
    cb_reg_op!(cb_sra_e, sra, e);
    cb_reg_op!(cb_sra_h, sra, h);
    cb_reg_op!(cb_sra_l, sra, l);
    cb_hlp_op!(cb_sra_hlp, sra);
    cb_reg_op!(cb_sra_a, sra, a);
    // SWAP r / SWAP (HL)
    cb_reg_op!(cb_swap_b, swap, b);
    cb_reg_op!(cb_swap_c, swap, c);
    cb_reg_op!(cb_swap_d, swap, d);
    cb_reg_op!(cb_swap_e, swap, e);
    cb_reg_op!(cb_swap_h, swap, h);
    cb_reg_op!(cb_swap_l, swap, l);
    cb_hlp_op!(cb_swap_hlp, swap);
    cb_reg_op!(cb_swap_a, swap, a);
    // SRL r / SRL (HL)
    cb_reg_op!(cb_srl_b, srl, b);
    cb_reg_op!(cb_srl_c, srl, c);
    cb_reg_op!(cb_srl_d, srl, d);
    cb_reg_op!(cb_srl_e, srl, e);
    cb_reg_op!(cb_srl_h, srl, h);
    cb_reg_op!(cb_srl_l, srl, l);
    cb_hlp_op!(cb_srl_hlp, srl);
    cb_reg_op!(cb_srl_a, srl, a);
    // BIT b, r / BIT b, (HL)
    cb_reg_op!(cb_bit_0_b, bit, 0, b);
    cb_reg_op!(cb_bit_0_c, bit, 0, c);
    cb_reg_op!(cb_bit_0_d, bit, 0, d);
    cb_reg_op!(cb_bit_0_e, bit, 0, e);
    cb_reg_op!(cb_bit_0_h, bit, 0, h);
    cb_reg_op!(cb_bit_0_l, bit, 0, l);
    cb_hlp_op!(cb_bit_0_hlp, bit, 0);
    cb_reg_op!(cb_bit_0_a, bit, 0, a);
    cb_reg_op!(cb_bit_1_b, bit, 1, b);
    cb_reg_op!(cb_bit_1_c, bit, 1, c);
    cb_reg_op!(cb_bit_1_d, bit, 1, d);
    cb_reg_op!(cb_bit_1_e, bit, 1, e);
    cb_reg_op!(cb_bit_1_h, bit, 1, h);
    cb_reg_op!(cb_bit_1_l, bit, 1, l);
    cb_hlp_op!(cb_bit_1_hlp, bit, 1);
    cb_reg_op!(cb_bit_1_a, bit, 1, a);
    cb_reg_op!(cb_bit_2_b, bit, 2, b);
    cb_reg_op!(cb_bit_2_c, bit, 2, c);
    cb_reg_op!(cb_bit_2_d, bit, 2, d);
    cb_reg_op!(cb_bit_2_e, bit, 2, e);
    cb_reg_op!(cb_bit_2_h, bit, 2, h);
    cb_reg_op!(cb_bit_2_l, bit, 2, l);
    cb_hlp_op!(cb_bit_2_hlp, bit, 2);
    cb_reg_op!(cb_bit_2_a, bit, 2, a);
    cb_reg_op!(cb_bit_3_b, bit, 3, b);
    cb_reg_op!(cb_bit_3_c, bit, 3, c);
    cb_reg_op!(cb_bit_3_d, bit, 3, d);
    cb_reg_op!(cb_bit_3_e, bit, 3, e);
    cb_reg_op!(cb_bit_3_h, bit, 3, h);
    cb_reg_op!(cb_bit_3_l, bit, 3, l);
    cb_hlp_op!(cb_bit_3_hlp, bit, 3);
    cb_reg_op!(cb_bit_3_a, bit, 3, a);
    cb_reg_op!(cb_bit_4_b, bit, 4, b);
    cb_reg_op!(cb_bit_4_c, bit, 4, c);
    cb_reg_op!(cb_bit_4_d, bit, 4, d);
    cb_reg_op!(cb_bit_4_e, bit, 4, e);
    cb_reg_op!(cb_bit_4_h, bit, 4, h);
    cb_reg_op!(cb_bit_4_l, bit, 4, l);
    cb_hlp_op!(cb_bit_4_hlp, bit, 4);
    cb_reg_op!(cb_bit_4_a, bit, 4, a);
    cb_reg_op!(cb_bit_5_b, bit, 5, b);
    cb_reg_op!(cb_bit_5_c, bit, 5, c);
    cb_reg_op!(cb_bit_5_d, bit, 5, d);
    cb_reg_op!(cb_bit_5_e, bit, 5, e);
    cb_reg_op!(cb_bit_5_h, bit, 5, h);
    cb_reg_op!(cb_bit_5_l, bit, 5, l);
    cb_hlp_op!(cb_bit_5_hlp, bit, 5);
    cb_reg_op!(cb_bit_5_a, bit, 5, a);
    cb_reg_op!(cb_bit_6_b, bit, 6, b);
    cb_reg_op!(cb_bit_6_c, bit, 6, c);
    cb_reg_op!(cb_bit_6_d, bit, 6, d);
    cb_reg_op!(cb_bit_6_e, bit, 6, e);
    cb_reg_op!(cb_bit_6_h, bit, 6, h);
    cb_reg_op!(cb_bit_6_l, bit, 6, l);
    cb_hlp_op!(cb_bit_6_hlp, bit, 6);
    cb_reg_op!(cb_bit_6_a, bit, 6, a);
    cb_reg_op!(cb_bit_7_b, bit, 7, b);
    cb_reg_op!(cb_bit_7_c, bit, 7, c);
    cb_reg_op!(cb_bit_7_d, bit, 7, d);
    cb_reg_op!(cb_bit_7_e, bit, 7, e);
    cb_reg_op!(cb_bit_7_h, bit, 7, h);
    cb_reg_op!(cb_bit_7_l, bit, 7, l);
    cb_hlp_op!(cb_bit_7_hlp, bit, 7);
    cb_reg_op!(cb_bit_7_a, bit, 7, a);
    // RES b, r / RES b, (HL)
    cb_reg_op!(cb_res_0_b, res, 0, b);
    cb_reg_op!(cb_res_0_c, res, 0, c);
    cb_reg_op!(cb_res_0_d, res, 0, d);
    cb_reg_op!(cb_res_0_e, res, 0, e);
    cb_reg_op!(cb_res_0_h, res, 0, h);
    cb_reg_op!(cb_res_0_l, res, 0, l);
    cb_hlp_op!(cb_res_0_hlp, res, 0);
    cb_reg_op!(cb_res_0_a, res, 0, a);
    cb_reg_op!(cb_res_1_b, res, 1, b);
    cb_reg_op!(cb_res_1_c, res, 1, c);
    cb_reg_op!(cb_res_1_d, res, 1, d);
    cb_reg_op!(cb_res_1_e, res, 1, e);
    cb_reg_op!(cb_res_1_h, res, 1, h);
    cb_reg_op!(cb_res_1_l, res, 1, l);
    cb_hlp_op!(cb_res_1_hlp, res, 1);
    cb_reg_op!(cb_res_1_a, res, 1, a);
    cb_reg_op!(cb_res_2_b, res, 2, b);
    cb_reg_op!(cb_res_2_c, res, 2, c);
    cb_reg_op!(cb_res_2_d, res, 2, d);
    cb_reg_op!(cb_res_2_e, res, 2, e);
    cb_reg_op!(cb_res_2_h, res, 2, h);
    cb_reg_op!(cb_res_2_l, res, 2, l);
    cb_hlp_op!(cb_res_2_hlp, res, 2);
    cb_reg_op!(cb_res_2_a, res, 2, a);
    cb_reg_op!(cb_res_3_b, res, 3, b);
    cb_reg_op!(cb_res_3_c, res, 3, c);
    cb_reg_op!(cb_res_3_d, res, 3, d);
    cb_reg_op!(cb_res_3_e, res, 3, e);
    cb_reg_op!(cb_res_3_h, res, 3, h);
    cb_reg_op!(cb_res_3_l, res, 3, l);
    cb_hlp_op!(cb_res_3_hlp, res, 3);
    cb_reg_op!(cb_res_3_a, res, 3, a);
    cb_reg_op!(cb_res_4_b, res, 4, b);
    cb_reg_op!(cb_res_4_c, res, 4, c);
    cb_reg_op!(cb_res_4_d, res, 4, d);
    cb_reg_op!(cb_res_4_e, res, 4, e);
    cb_reg_op!(cb_res_4_h, res, 4, h);
    cb_reg_op!(cb_res_4_l, res, 4, l);
    cb_hlp_op!(cb_res_4_hlp, res, 4);
    cb_reg_op!(cb_res_4_a, res, 4, a);
    cb_reg_op!(cb_res_5_b, res, 5, b);
    cb_reg_op!(cb_res_5_c, res, 5, c);
    cb_reg_op!(cb_res_5_d, res, 5, d);
    cb_reg_op!(cb_res_5_e, res, 5, e);
    cb_reg_op!(cb_res_5_h, res, 5, h);
    cb_reg_op!(cb_res_5_l, res, 5, l);
    cb_hlp_op!(cb_res_5_hlp, res, 5);
    cb_reg_op!(cb_res_5_a, res, 5, a);
    cb_reg_op!(cb_res_6_b, res, 6, b);
    cb_reg_op!(cb_res_6_c, res, 6, c);
    cb_reg_op!(cb_res_6_d, res, 6, d);
    cb_reg_op!(cb_res_6_e, res, 6, e);
    cb_reg_op!(cb_res_6_h, res, 6, h);
    cb_reg_op!(cb_res_6_l, res, 6, l);
    cb_hlp_op!(cb_res_6_hlp, res, 6);
    cb_reg_op!(cb_res_6_a, res, 6, a);
    cb_reg_op!(cb_res_7_b, res, 7, b);
    cb_reg_op!(cb_res_7_c, res, 7, c);
    cb_reg_op!(cb_res_7_d, res, 7, d);
    cb_reg_op!(cb_res_7_e, res, 7, e);
    cb_reg_op!(cb_res_7_h, res, 7, h);
    cb_reg_op!(cb_res_7_l, res, 7, l);
    cb_hlp_op!(cb_res_7_hlp, res, 7);
    cb_reg_op!(cb_res_7_a, res, 7, a);
    // SET b, r / SET b, (HL)
    cb_reg_op!(cb_set_0_b, set, 0, b);
    cb_reg_op!(cb_set_0_c, set, 0, c);
    cb_reg_op!(cb_set_0_d, set, 0, d);
    cb_reg_op!(cb_set_0_e, set, 0, e);
    cb_reg_op!(cb_set_0_h, set, 0, h);
    cb_reg_op!(cb_set_0_l, set, 0, l);
    cb_hlp_op!(cb_set_0_hlp, set, 0);
    cb_reg_op!(cb_set_0_a, set, 0, a);
    cb_reg_op!(cb_set_1_b, set, 1, b);
    cb_reg_op!(cb_set_1_c, set, 1, c);
    cb_reg_op!(cb_set_1_d, set, 1, d);
    cb_reg_op!(cb_set_1_e, set, 1, e);
    cb_reg_op!(cb_set_1_h, set, 1, h);
    cb_reg_op!(cb_set_1_l, set, 1, l);
    cb_hlp_op!(cb_set_1_hlp, set, 1);
    cb_reg_op!(cb_set_1_a, set, 1, a);
    cb_reg_op!(cb_set_2_b, set, 2, b);
    cb_reg_op!(cb_set_2_c, set, 2, c);
    cb_reg_op!(cb_set_2_d, set, 2, d);
    cb_reg_op!(cb_set_2_e, set, 2, e);
    cb_reg_op!(cb_set_2_h, set, 2, h);
    cb_reg_op!(cb_set_2_l, set, 2, l);
    cb_hlp_op!(cb_set_2_hlp, set, 2);
    cb_reg_op!(cb_set_2_a, set, 2, a);
    cb_reg_op!(cb_set_3_b, set, 3, b);
    cb_reg_op!(cb_set_3_c, set, 3, c);
    cb_reg_op!(cb_set_3_d, set, 3, d);
    cb_reg_op!(cb_set_3_e, set, 3, e);
    cb_reg_op!(cb_set_3_h, set, 3, h);
    cb_reg_op!(cb_set_3_l, set, 3, l);
    cb_hlp_op!(cb_set_3_hlp, set, 3);
    cb_reg_op!(cb_set_3_a, set, 3, a);
    cb_reg_op!(cb_set_4_b, set, 4, b);
    cb_reg_op!(cb_set_4_c, set, 4, c);
    cb_reg_op!(cb_set_4_d, set, 4, d);
    cb_reg_op!(cb_set_4_e, set, 4, e);
    cb_reg_op!(cb_set_4_h, set, 4, h);
    cb_reg_op!(cb_set_4_l, set, 4, l);
    cb_hlp_op!(cb_set_4_hlp, set, 4);
    cb_reg_op!(cb_set_4_a, set, 4, a);
    cb_reg_op!(cb_set_5_b, set, 5, b);
    cb_reg_op!(cb_set_5_c, set, 5, c);
    cb_reg_op!(cb_set_5_d, set, 5, d);
    cb_reg_op!(cb_set_5_e, set, 5, e);
    cb_reg_op!(cb_set_5_h, set, 5, h);
    cb_reg_op!(cb_set_5_l, set, 5, l);
    cb_hlp_op!(cb_set_5_hlp, set, 5);
    cb_reg_op!(cb_set_5_a, set, 5, a);
    cb_reg_op!(cb_set_6_b, set, 6, b);
    cb_reg_op!(cb_set_6_c, set, 6, c);
    cb_reg_op!(cb_set_6_d, set, 6, d);
    cb_reg_op!(cb_set_6_e, set, 6, e);
    cb_reg_op!(cb_set_6_h, set, 6, h);
    cb_reg_op!(cb_set_6_l, set, 6, l);
    cb_hlp_op!(cb_set_6_hlp, set, 6);
    cb_reg_op!(cb_set_6_a, set, 6, a);
    cb_reg_op!(cb_set_7_b, set, 7, b);
    cb_reg_op!(cb_set_7_c, set, 7, c);
    cb_reg_op!(cb_set_7_d, set, 7, d);
    cb_reg_op!(cb_set_7_e, set, 7, e);
    cb_reg_op!(cb_set_7_h, set, 7, h);
    cb_reg_op!(cb_set_7_l, set, 7, l);
    cb_hlp_op!(cb_set_7_hlp, set, 7);
    cb_reg_op!(cb_set_7_a, set, 7, a);
} // End impl Cpu
