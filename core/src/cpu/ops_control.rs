use super::{Cpu, CpuResult, constants::*};
use crate::instruction::CB_INSTRUCTIONS;
use crate::memory_bus::MemoryBus;
use crate::memory_map;
use log;

// --- Control Flow Implementations ---
impl Cpu {
    // NOP
    pub fn op_nop(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        Ok(0)
    }

    // JP a16 / JP HL / JP cc, a16
    fn conditional_jp_a16(&mut self, condition: bool, bus: &MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        if condition {
            self.pc = addr;
            Ok(4) // Branch taken costs 4 extra cycles (total 16)
        } else {
            Ok(0) // Branch not taken costs 0 extra cycles (total 12)
        }
    }
    pub fn op_jp_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.read_d16(bus);
        Ok(0) // Unconditional JP takes 16 base cycles
    }
    pub fn op_jp_hl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.get_hl();
        Ok(0) // JP HL takes 4 base cycles
    }
    pub fn op_jp_nz_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(!self.get_flag(FLAG_Z), bus)
    }
    pub fn op_jp_z_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(self.get_flag(FLAG_Z), bus)
    }
    pub fn op_jp_nc_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(!self.get_flag(FLAG_C), bus)
    }
    pub fn op_jp_c_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jp_a16(self.get_flag(FLAG_C), bus)
    }

    // JR r8 / JR cc, r8
    fn conditional_jr(&mut self, condition: bool, bus: &MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        if condition {
            self.pc = self.pc.wrapping_add(offset as i16 as u16);
            Ok(4) // Branch taken costs 4 extra cycles (total 12)
        } else {
            Ok(0) // Branch not taken costs 0 extra cycles (total 8)
        }
    }
    pub fn op_jr_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let offset = self.read_r8(bus);
        self.pc = self.pc.wrapping_add(offset as i16 as u16);
        Ok(0) // Unconditional JR takes 12 base cycles
    }
    pub fn op_jr_nz_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jr(!self.get_flag(FLAG_Z), bus)
    }
    pub fn op_jr_z_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jr(self.get_flag(FLAG_Z), bus)
    }
    pub fn op_jr_nc_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jr(!self.get_flag(FLAG_C), bus)
    }
    pub fn op_jr_c_r8(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_jr(self.get_flag(FLAG_C), bus)
    }

    // CALL a16 / CALL cc, a16
    fn conditional_call_a16(&mut self, condition: bool, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        if condition {
            self.push_word(self.pc, bus); // Push address *after* CALL instruction
            self.pc = addr;
            Ok(12) // Branch taken costs 12 extra cycles (total 24)
        } else {
            Ok(0) // Branch not taken costs 0 extra cycles (total 12)
        }
    }
    pub fn op_call_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let addr = self.read_d16(bus);
        self.push_word(self.pc, bus);
        self.pc = addr;
        Ok(0) // Unconditional CALL takes 24 base cycles
    }
    pub fn op_call_nz_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(!self.get_flag(FLAG_Z), bus)
    }
    pub fn op_call_z_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(self.get_flag(FLAG_Z), bus)
    }
    pub fn op_call_nc_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(!self.get_flag(FLAG_C), bus)
    }
    pub fn op_call_c_a16(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_call_a16(self.get_flag(FLAG_C), bus)
    }

    // RET / RET cc / RETI
    fn conditional_ret(&mut self, condition: bool, bus: &mut MemoryBus) -> CpuResult<u16> {
        if condition {
            self.pc = self.pop_word(bus);
            Ok(12) // Branch taken costs 12 extra cycles (total 20)
        } else {
            Ok(0) // Branch not taken costs 0 extra cycles (total 8)
        }
    }
    pub fn op_ret(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.pop_word(bus);
        Ok(0) // Unconditional RET takes 16 base cycles
    }
    pub fn op_ret_nz(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(!self.get_flag(FLAG_Z), bus)
    }
    pub fn op_ret_z(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(self.get_flag(FLAG_Z), bus)
    }
    pub fn op_ret_nc(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(!self.get_flag(FLAG_C), bus)
    }
    pub fn op_ret_c(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.conditional_ret(self.get_flag(FLAG_C), bus)
    }
    pub fn op_reti(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.pc = self.pop_word(bus);
        self.ime = true;
        self.ime_scheduled = false;
        Ok(0) // RETI takes 16 base cycles
    }

    // RST n
    fn rst(&mut self, vector: u16, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.push_word(self.pc, bus);
        self.pc = vector;
        Ok(0) // RST takes 16 base cycles
    }
    pub fn op_rst_00h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0000, bus)
    }
    pub fn op_rst_08h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0008, bus)
    }
    pub fn op_rst_10h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0010, bus)
    }
    pub fn op_rst_18h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0018, bus)
    }
    pub fn op_rst_20h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0020, bus)
    }
    pub fn op_rst_28h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0028, bus)
    }
    pub fn op_rst_30h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0030, bus)
    }
    pub fn op_rst_38h(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        self.rst(0x0038, bus)
    }

    // Misc Control
    pub fn op_di(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.ime = false;
        self.ime_scheduled = false;
        Ok(0)
    }
    pub fn op_ei(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.ime_scheduled = true;
        Ok(0)
    }
    pub fn op_halt(&mut self, memory_bus: &mut MemoryBus) -> CpuResult<u16> {
        let ie = memory_bus.read_byte(memory_map::INTERRUPT_ENABLE_REGISTER);
        let iflags = memory_bus.read_byte(memory_map::IF_ADDR);
        if !self.ime && (ie & iflags & 0x1F) != 0 {
            log::warn!(
                "HALT bug triggered at PC={:#06X}! IME=0, IE&IF={:02X}. Next instruction will execute.",
                self.instruction_pc,
                ie & iflags & 0x1F
            );
            // PC already incremented, effectively skipping HALT cycle.
        } else {
            self.halted = true;
        }
        Ok(0)
    }
    pub fn op_stop(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        // For now, just flag it. Proper STOP needs more handling (power modes, CGB speed switch).
        self.stop_requested = true;
        log::warn!(
            "STOP instruction encountered at PC={:#06X} (behavior may be incomplete)",
            self.instruction_pc
        );
        Ok(0)
    }

    // Flags
    pub fn op_scf(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, true);
        Ok(0)
    }
    pub fn op_ccf(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        let current_c = self.get_flag(FLAG_C);
        self.set_flag(FLAG_N | FLAG_H, false);
        self.set_flag(FLAG_C, !current_c);
        Ok(0)
    }

    // Misc ALU/Data
    pub fn op_cpl(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = !self.a;
        self.set_flag(FLAG_N | FLAG_H, true);
        Ok(0)
    }
    pub fn op_daa(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.daa();
        Ok(0)
    }

    // --- CB Prefix Dispatcher ---
    pub fn op_prefix_cb(&mut self, bus: &mut MemoryBus) -> CpuResult<u16> {
        let cb_opcode = bus.read_byte(self.instruction_pc.wrapping_add(1));
        let cb_instr = &CB_INSTRUCTIONS[cb_opcode as usize];
        // Execute the specific CB function (defined in ops_cb.rs)
        let cb_result = (cb_instr.execute)(self, bus);
        match cb_result {
            // Return cycles for the specific CB op *only*
            Ok(_) => Ok(cb_instr.cycles as u16),
            Err(e) => Err(format!("CB Opcode {:#04X} Error: {}", cb_opcode, e)),
        }
    }

    // --- Invalid Opcode Handler ---
    pub fn handle_invalid_opcode(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        // Error will be logged by the main step loop
        Err("Invalid/Unknown Opcode encountered".to_string())
    }
}
