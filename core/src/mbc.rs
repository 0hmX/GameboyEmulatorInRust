// src/mbc.rs

/// Defines the Memory Bank Controller type used by the cartridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MbcType {
    NoMbc,
    Mbc1,
    Mbc3,
    // Future: Mbc2, Mbc5, etc.
}

impl MbcType {
    /// Determines MBC Type, RAM presence, and Battery presence from the cartridge type code.
    pub fn from_header(cartridge_type_code: u8) -> (Self, bool, bool) {
        match cartridge_type_code {
            0x00 => (MbcType::NoMbc, false, false),
            0x01 => (MbcType::Mbc1, false, false),
            0x02 => (MbcType::Mbc1, true, false),
            0x03 => (MbcType::Mbc1, true, true),
            // 0x05 | 0x06 => panic!("MBC2 not implemented"), // Placeholder
            0x08 => (MbcType::NoMbc, true, false), // ROM+RAM
            0x09 => (MbcType::NoMbc, true, true),  // ROM+RAM+BATT
            0x0F => (MbcType::Mbc3, false, true),  // MBC3+TIMER+BATT
            0x10 => (MbcType::Mbc3, true, true),   // MBC3+TIMER+RAM+BATT
            0x11 => (MbcType::Mbc3, false, false), // MBC3
            0x12 => (MbcType::Mbc3, true, false),  // MBC3+RAM
            0x13 => (MbcType::Mbc3, true, true),   // MBC3+RAM+BATT
            // 0x19..=0x1E => panic!("MBC5 not implemented"), // Placeholder
            _ => panic!("Unsupported cartridge type: {:02X}", cartridge_type_code),
        }
    }
}