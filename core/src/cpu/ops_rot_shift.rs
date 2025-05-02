use super::{Cpu, CpuResult, constants::*};
use crate::memory_bus::MemoryBus;

// --- Rotate/Shift Implementations (Non-CB prefixed) ---
impl Cpu {
    // RLCA
    pub fn op_rlca(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rlc(self.a);
        self.set_flag(FLAG_Z, false); // Special case: Z flag is cleared
        Ok(0)
    }
    // RLA
    pub fn op_rla(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rl(self.a);
        self.set_flag(FLAG_Z, false); // Special case: Z flag is cleared
        Ok(0)
    }
    // RRCA
    pub fn op_rrca(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rrc(self.a);
        self.set_flag(FLAG_Z, false); // Special case: Z flag is cleared
        Ok(0)
    }
    // RRA
    pub fn op_rra(&mut self, _bus: &mut MemoryBus) -> CpuResult<u16> {
        self.a = self.rr(self.a);
        self.set_flag(FLAG_Z, false); // Special case: Z flag is cleared
        Ok(0)
    }
}