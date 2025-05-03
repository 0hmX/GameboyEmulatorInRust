use std::time::{SystemTime, UNIX_EPOCH};

/// Represents the Real-Time Clock registers for MBC3.
#[derive(Clone, Debug, Default)]
pub struct RtcRegisters {
    seconds: u8, // 0x08 (0-59)
    minutes: u8, // 0x09 (0-59)
    hours: u8,   // 0x0A (0-23)
    dl: u8,      // 0x0B (Lower 8 bits of day counter)
    dh: u8,      // 0x0C (Upper 1 bit of day counter + flags)

    // Internal state for timing based on system clock (simplification)
    last_updated_secs: u64,
}

impl RtcRegisters {
    const DAY_CARRY_BIT: u8 = 0b0000_0001; // Bit 0: Day Counter Carry Bit (1=Counter overflowed)
    const HALT_BIT: u8 = 0b0100_0000; // Bit 6: Halt (0=Active, 1=Stop Timer)
    const DAY_OVERFLOW_BIT: u8 = 0b1000_0000; // Bit 7: Day Counter Overflow (Read Only?)

    /// Creates a new RTC register set, initializing last update time.
    pub fn new() -> Self {
        let mut rtc = RtcRegisters::default();
        // Initialize last updated time to now
        rtc.last_updated_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        rtc
    }


    // Basic update based on system time - A real emulator might tie this to internal clock cycles
    pub fn update(&mut self) {
        if (self.dh & RtcRegisters::HALT_BIT) != 0 {
            self.last_updated_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return; // Timer halted, just update last_updated_secs
        }

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed_secs = now_secs.saturating_sub(self.last_updated_secs);

        if elapsed_secs == 0 {
            return; // No time passed
        }

        self.last_updated_secs = now_secs;

        // Cascade updates through seconds, minutes, hours, days
        let total_seconds = u64::from(self.seconds) + elapsed_secs;
        self.seconds = (total_seconds % 60) as u8;

        let total_minutes = u64::from(self.minutes) + (total_seconds / 60);
        self.minutes = (total_minutes % 60) as u8;

        let total_hours = u64::from(self.hours) + (total_minutes / 60);
        self.hours = (total_hours % 24) as u8;

        // Handle day counter (9 bits total: DH bit 0 + DL)
        let mut days = u64::from(self.dl) | (u64::from(self.dh & RtcRegisters::DAY_CARRY_BIT) << 8);
        days += total_hours / 24;

        if days >= 512 {
            // Day counter wraps around after 511 days
            days %= 512;
            self.dh |= RtcRegisters::DAY_OVERFLOW_BIT; // Set overflow flag
        } else {
            // Clear overflow flag if it was somehow set and we are no longer overflowed
            // Note: Real hardware might require explicit clearing? Check Pan Docs. Assume it clears for now.
            // self.dh &= !RtcRegisters::DAY_OVERFLOW_BIT;
        }

        self.dl = (days & 0xFF) as u8;
        // Update DH: Preserve Halt bit, clear old carry, set new carry from bit 8 of days
        self.dh = (self.dh & RtcRegisters::HALT_BIT) | // Keep Halt bit
                  ((days >> 8) as u8 & RtcRegisters::DAY_CARRY_BIT) | // Set new Carry bit
                  (self.dh & RtcRegisters::DAY_OVERFLOW_BIT); // Keep potentially set Overflow bit
    }

    /// Reads the value of a selected RTC register.
    pub fn read(&self, reg_select: u8) -> u8 {
        match reg_select {
            0x08 => self.seconds,
            0x09 => self.minutes,
            0x0A => self.hours,
            0x0B => self.dl,
            0x0C => self.dh,
            _ => 0xFF, // Invalid RTC register selection
        }
    }

    /// Writes a value to a selected RTC register.
    pub fn write(&mut self, reg_select: u8, value: u8) {
        match reg_select {
            0x08 => self.seconds = value.min(59), // Clamp to valid range
            0x09 => self.minutes = value.min(59),
            0x0A => self.hours = value.min(23),
            0x0B => self.dl = value, // Full 8 bits writeable
            0x0C => {
                // Only Day Carry (bit 0) and Halt (bit 6) are writeable
                self.dh = (value & (RtcRegisters::DAY_CARRY_BIT | RtcRegisters::HALT_BIT))
                    | (self.dh & RtcRegisters::DAY_OVERFLOW_BIT); // Preserve read-only overflow bit
            }
            _ => {} // Invalid RTC register selection
        }
    }
}