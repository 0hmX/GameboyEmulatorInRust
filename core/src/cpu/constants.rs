// Flag Positions (Bit index in F register)
pub const FLAG_Z_POS: u8 = 7; // Zero Flag
pub const FLAG_N_POS: u8 = 6; // Subtract Flag
pub const FLAG_H_POS: u8 = 5; // Half Carry Flag
pub const FLAG_C_POS: u8 = 4; // Carry Flag

// Flag Masks (Bit masks for F register)
pub const FLAG_Z: u8 = 1 << FLAG_Z_POS;
pub const FLAG_N: u8 = 1 << FLAG_N_POS;
pub const FLAG_H: u8 = 1 << FLAG_H_POS;
pub const FLAG_C: u8 = 1 << FLAG_C_POS;

// Interrupt Vectors (Jump addresses for interrupt service routines)
pub const VBLANK_VECTOR: u16 = 0x0040;
pub const LCD_STAT_VECTOR: u16 = 0x0048;
pub const TIMER_VECTOR: u16 = 0x0050;
pub const SERIAL_VECTOR: u16 = 0x0058;
pub const JOYPAD_VECTOR: u16 = 0x0060;

// Related Memory Addresses (defined in memory_map.rs, but useful here for context)
// pub use crate::memory_map::{INTERRUPT_ENABLE_REGISTER as IE_REGISTER, IF_ADDR as IF_REGISTER};
// Note: It's generally better to import these from memory_map where needed,
// rather than redefining or re-exporting them here to avoid duplication.
// We'll use the direct addresses from memory_map in the cpu code.