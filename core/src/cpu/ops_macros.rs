// --- LD Macros ---
macro_rules! ld_r_r {
    ($name:ident, $r1:ident, $r2:ident) => {
        #[inline(always)]
        pub fn $name(&mut self, _bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.$r1 = self.$r2;
            Ok(0)
        }
    };
}
macro_rules! ld_r_hlp {
    ($name:ident, $r1:ident) => {
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.$r1 = bus.read_byte(self.get_hl());
            Ok(0)
        }
    };
}
macro_rules! ld_hlp_r {
    ($name:ident, $r2:ident) => {
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            bus.write_byte(self.get_hl(), self.$r2);
            Ok(0)
        }
    };
}

// --- ALU Macros ---
macro_rules! alu_a_r {
    ($name:ident, $op:ident, $r2:ident) => { // No carry version
        #[inline(always)]
        pub fn $name(&mut self, _bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.$op(self.$r2, false);
            Ok(0)
        }
    };
    ($name:ident, $op:ident, $r2:ident, carry) => { // With carry version
         #[inline(always)]
        pub fn $name(&mut self, _bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.$op(self.$r2, true);
            Ok(0)
        }
    };
}
macro_rules! alu_a_hlp {
    ($name:ident, $op:ident) => { // No carry version
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            let addr = self.get_hl();
            let val = bus.read_byte(addr);
            self.$op(val, false);
            Ok(0)
        }
    };
    ($name:ident, $op:ident, carry) => { // With carry version
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            let addr = self.get_hl();
            let val = bus.read_byte(addr);
            self.$op(val, true);
            Ok(0)
        }
    };
}

macro_rules! cb_reg_op {
    ($name:ident, $op:ident, $reg:ident) => { // Bitwise op
        #[inline(always)]
        pub fn $name(&mut self, _bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.$reg = self.$op(self.$reg);
            Ok(0)
        }
    };
    ($name:ident, bit, $bit:expr, $reg:ident) => { // BIT op
        #[inline(always)]
        pub fn $name(&mut self, _bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.op_bit($bit, self.$reg);
            Ok(0)
        }
    };
    ($name:ident, res, $bit:expr, $reg:ident) => { // RES op
        #[inline(always)]
        pub fn $name(&mut self, _bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.$reg &= !(1 << $bit);
            Ok(0)
        }
    };
    ($name:ident, set, $bit:expr, $reg:ident) => { // SET op
         #[inline(always)]
        pub fn $name(&mut self, _bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            self.$reg |= (1 << $bit);
            Ok(0)
        }
    };
}
macro_rules! cb_hlp_op {
    ($name:ident, $op:ident) => { // Bitwise op on (HL)
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            let addr = self.get_hl();
            let value = bus.read_byte(addr);
            let result = self.$op(value);
            bus.write_byte(addr, result);
            Ok(0)
        }
    };
    ($name:ident, bit, $bit:expr) => { // BIT op on (HL)
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            let value = bus.read_byte(self.get_hl());
            self.op_bit($bit, value);
            Ok(0)
        }
    };
    ($name:ident, res, $bit:expr) => { // RES op on (HL)
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            let addr = self.get_hl();
            let value = bus.read_byte(addr);
            let result = value & !(1 << $bit);
            bus.write_byte(addr, result);
            Ok(0)
        }
    };
    ($name:ident, set, $bit:expr) => { // SET op on (HL)
        pub fn $name(&mut self, bus: &mut crate::memory_bus::MemoryBus) -> super::CpuResult<u16> {
            let addr = self.get_hl();
            let value = bus.read_byte(addr);
            let result = value | (1 << $bit);
            bus.write_byte(addr, result);
            Ok(0)
        }
    };
}