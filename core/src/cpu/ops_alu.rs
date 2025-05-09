// src/cpu/ops_alu.rs

use super::{Cpu, CpuResult, constants::*};
use crate::memory_bus::MemoryBus;

// --- ALU Implementations ---
impl Cpu {
    // ADD A, r / ADD A, (HL) / ADD A, d8 (Generated by macro + specific)
    alu_a_r!(op_add_a_b, add_a, b);
    alu_a_r!(op_add_a_c, add_a, c);
    alu_a_r!(op_add_a_d, add_a, d);
    alu_a_r!(op_add_a_e, add_a, e);
    alu_a_r!(op_add_a_h, add_a, h);
    alu_a_r!(op_add_a_l, add_a, l);
    alu_a_hlp!(op_add_a_hlp, add_a);
    alu_a_r!(op_add_a_a, add_a, a);
    pub fn op_add_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_a(self.read_d8(bus), false);
        Ok(0)
    }

    // ADC A, r / ADC A, (HL) / ADC A, d8 (Generated by macro + specific)
    alu_a_r!(op_adc_a_b, add_a, b, carry);
    alu_a_r!(op_adc_a_c, add_a, c, carry);
    alu_a_r!(op_adc_a_d, add_a, d, carry);
    alu_a_r!(op_adc_a_e, add_a, e, carry);
    alu_a_r!(op_adc_a_h, add_a, h, carry);
    alu_a_r!(op_adc_a_l, add_a, l, carry);
    alu_a_hlp!(op_adc_a_hlp, add_a, carry);
    alu_a_r!(op_adc_a_a, add_a, a, carry);
    pub fn op_adc_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_a(self.read_d8(bus), true);
        Ok(0)
    }

    // SUB A, r / SUB A, (HL) / SUB A, d8 (Generated by macro + specific)
    alu_a_r!(op_sub_a_b, sub_a, b);
    alu_a_r!(op_sub_a_c, sub_a, c);
    alu_a_r!(op_sub_a_d, sub_a, d);
    alu_a_r!(op_sub_a_e, sub_a, e);
    alu_a_r!(op_sub_a_h, sub_a, h);
    alu_a_r!(op_sub_a_l, sub_a, l);
    alu_a_hlp!(op_sub_a_hlp, sub_a);
    alu_a_r!(op_sub_a_a, sub_a, a);
    pub fn op_sub_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sub_a(self.read_d8(bus), false);
        Ok(0)
    }

    // SBC A, r / SBC A, (HL) / SBC A, d8 (Generated by macro + specific)
    alu_a_r!(op_sbc_a_b, sub_a, b, carry);
    alu_a_r!(op_sbc_a_c, sub_a, c, carry);
    alu_a_r!(op_sbc_a_d, sub_a, d, carry);
    alu_a_r!(op_sbc_a_e, sub_a, e, carry);
    alu_a_r!(op_sbc_a_h, sub_a, h, carry);
    alu_a_r!(op_sbc_a_l, sub_a, l, carry);
    alu_a_hlp!(op_sbc_a_hlp, sub_a, carry);
    alu_a_r!(op_sbc_a_a, sub_a, a, carry);
    pub fn op_sbc_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sub_a(self.read_d8(bus), true);
        Ok(0)
    }

    // AND A, r / AND A, (HL) / AND A, d8 (Generated by macro + specific)
    alu_a_r!(op_and_a_b, and_a, b);
    alu_a_r!(op_and_a_c, and_a, c);
    alu_a_r!(op_and_a_d, and_a, d);
    alu_a_r!(op_and_a_e, and_a, e);
    alu_a_r!(op_and_a_h, and_a, h);
    alu_a_r!(op_and_a_l, and_a, l);
    alu_a_hlp!(op_and_a_hlp, and_a);
    alu_a_r!(op_and_a_a, and_a, a);
    pub fn op_and_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.and_a(self.read_d8(bus), false);
        Ok(0)
    }

    // XOR A, r / XOR A, (HL) / XOR A, d8 (Generated by macro + specific)
    alu_a_r!(op_xor_a_b, xor_a, b);
    alu_a_r!(op_xor_a_c, xor_a, c);
    alu_a_r!(op_xor_a_d, xor_a, d);
    alu_a_r!(op_xor_a_e, xor_a, e);
    alu_a_r!(op_xor_a_h, xor_a, h);
    alu_a_r!(op_xor_a_l, xor_a, l);
    alu_a_hlp!(op_xor_a_hlp, xor_a);
    alu_a_r!(op_xor_a_a, xor_a, a);
    pub fn op_xor_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.xor_a(self.read_d8(bus), false);
        Ok(0)
    }

    // OR A, r / OR A, (HL) / OR A, d8 (Generated by macro + specific)
    alu_a_r!(op_or_a_b, or_a, b);
    alu_a_r!(op_or_a_c, or_a, c);
    alu_a_r!(op_or_a_d, or_a, d);
    alu_a_r!(op_or_a_e, or_a, e);
    alu_a_r!(op_or_a_h, or_a, h);
    alu_a_r!(op_or_a_l, or_a, l);
    alu_a_hlp!(op_or_a_hlp, or_a);
    alu_a_r!(op_or_a_a, or_a, a);
    pub fn op_or_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.or_a(self.read_d8(bus), false);
        Ok(0)
    }

    // CP A, r / CP A, (HL) / CP A, d8 (Generated by macro + specific)
    alu_a_r!(op_cp_a_b, cp_a, b);
    alu_a_r!(op_cp_a_c, cp_a, c);
    alu_a_r!(op_cp_a_d, cp_a, d);
    alu_a_r!(op_cp_a_e, cp_a, e);
    alu_a_r!(op_cp_a_h, cp_a, h);
    alu_a_r!(op_cp_a_l, cp_a, l);
    alu_a_hlp!(op_cp_a_hlp, cp_a);
    alu_a_r!(op_cp_a_a, cp_a, a);
    pub fn op_cp_a_d8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.cp_a(self.read_d8(bus), false);
        Ok(0)
    }

    // INC r / INC (HL)
    pub fn op_inc_b(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.b = self.inc_u8(self.b);
        Ok(0)
    }
    pub fn op_inc_c(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.c = self.inc_u8(self.c);
        Ok(0)
    }
    pub fn op_inc_d(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.d = self.inc_u8(self.d);
        Ok(0)
    }
    pub fn op_inc_e(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.e = self.inc_u8(self.e);
        Ok(0)
    }
    pub fn op_inc_h(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.h = self.inc_u8(self.h);
        Ok(0)
    }
    pub fn op_inc_l(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.l = self.inc_u8(self.l);
        Ok(0)
    }
    pub fn op_inc_a(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.inc_u8(self.a);
        Ok(0)
    }
    pub fn op_inc_hlp(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        let value = bus.read_byte(addr);
        let result = self.inc_u8(value);
        bus.write_byte(addr, result);
        Ok(0)
    }

    // DEC r / DEC (HL)
    pub fn op_dec_b(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.b = self.dec_u8(self.b);
        Ok(0)
    }
    pub fn op_dec_c(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.c = self.dec_u8(self.c);
        Ok(0)
    }
    pub fn op_dec_d(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.d = self.dec_u8(self.d);
        Ok(0)
    }
    pub fn op_dec_e(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.e = self.dec_u8(self.e);
        Ok(0)
    }
    pub fn op_dec_h(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.h = self.dec_u8(self.h);
        Ok(0)
    }
    pub fn op_dec_l(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.l = self.dec_u8(self.l);
        Ok(0)
    }
    pub fn op_dec_a(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.dec_u8(self.a);
        Ok(0)
    }
    pub fn op_dec_hlp(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.get_hl();
        let value = bus.read_byte(addr);
        let result = self.dec_u8(value);
        bus.write_byte(addr, result);
        Ok(0)
    }

    // ADD HL, rr / ADD HL, SP
    pub fn op_add_hl_bc(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.get_bc());
        Ok(0)
    }
    pub fn op_add_hl_de(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.get_de());
        Ok(0)
    }
    pub fn op_add_hl_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.get_hl());
        Ok(0)
    }
    pub fn op_add_hl_sp(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.add_hl(self.sp);
        Ok(0)
    }

    // ADD SP, r8
    pub fn op_add_sp_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        let value = offset as i16 as u16; // Sign extend
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

    // INC rr / INC SP
    pub fn op_inc_bc(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_bc(self.get_bc().wrapping_add(1));
        Ok(0)
    }
    pub fn op_inc_de(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_de(self.get_de().wrapping_add(1));
        Ok(0)
    }
    pub fn op_inc_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_hl(self.get_hl().wrapping_add(1));
        Ok(0)
    }
    pub fn op_inc_sp(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sp = self.sp.wrapping_add(1);
        Ok(0)
    }

    // DEC rr / DEC SP
    pub fn op_dec_bc(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_bc(self.get_bc().wrapping_sub(1));
        Ok(0)
    }
    pub fn op_dec_de(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_de(self.get_de().wrapping_sub(1));
        Ok(0)
    }
    pub fn op_dec_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_hl(self.get_hl().wrapping_sub(1));
        Ok(0)
    }
    pub fn op_dec_sp(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.sp = self.sp.wrapping_sub(1);
        Ok(0)
    }
}
