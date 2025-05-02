use sdl2::keyboard::Keycode; // For input mapping in key_down/key_up
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// --- Memory Map Constants ---
const ROM_BANK_0_START: u16 = 0x0000;
const ROM_BANK_0_END: u16 = 0x3FFF;
const ROM_BANK_0_SIZE: usize = 0x4000; // 16 KiB

const ROM_BANK_N_START: u16 = 0x4000;
const ROM_BANK_N_END: u16 = 0x7FFF;
const ROM_BANK_N_SIZE: usize = 0x4000; // 16 KiB

const VRAM_START: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;
const VRAM_SIZE: usize = 0x2000; // 8 KiB

const EXT_RAM_START: u16 = 0xA000;
const EXT_RAM_END: u16 = 0xBFFF;
const EXT_RAM_SIZE: usize = 0x2000; // 8 KiB (per bank)

const WRAM_BANK_0_START: u16 = 0xC000;
const WRAM_BANK_0_END: u16 = 0xCFFF;
const WRAM_BANK_0_SIZE: usize = 0x1000; // 4 KiB

const WRAM_BANK_N_START: u16 = 0xD000;
const WRAM_BANK_N_END: u16 = 0xDFFF;
const WRAM_BANK_N_SIZE: usize = 0x1000; // 4 KiB

const ECHO_RAM_START: u16 = 0xE000;
const ECHO_RAM_END: u16 = 0xFDFF;

const OAM_START: u16 = 0xFE00;
const OAM_END: u16 = 0xFE9F;
const OAM_SIZE: usize = 0xA0; // 160 bytes

const NOT_USABLE_START: u16 = 0xFEA0;
const NOT_USABLE_END: u16 = 0xFEFF;

const IO_REGISTERS_START: u16 = 0xFF00;
const IO_REGISTERS_END: u16 = 0xFF7F;
const IO_REGISTERS_SIZE: usize = 0x80; // 128 bytes

const HRAM_START: u16 = 0xFF80;
const HRAM_END: u16 = 0xFFFE;
const HRAM_SIZE: usize = 0x7F; // 127 bytes

const INTERRUPT_ENABLE_REGISTER: u16 = 0xFFFF;

// --- Specific I/O Register Addresses ---
const P1_JOYP_ADDR: u16 = 0xFF00; // Joypad input register
const SB_ADDR: u16 = 0xFF01; // Serial transfer data
const SC_ADDR: u16 = 0xFF02; // Serial transfer control
const DIV_ADDR: u16 = 0xFF04; // Divider Register
const TIMA_ADDR: u16 = 0xFF05; // Timer counter
const TMA_ADDR: u16 = 0xFF06; // Timer Modulo
const TAC_ADDR: u16 = 0xFF07; // Timer Control
const IF_ADDR: u16 = 0xFF0F; // Interrupt Flag register
const NR10_ADDR: u16 = 0xFF10; // Sound channel 1 sweep
// ... add other sound registers FF11-FF26
const LCDC_ADDR: u16 = 0xFF40; // LCD Control
const STAT_ADDR: u16 = 0xFF41; // LCD Status
const SCY_ADDR: u16 = 0xFF42; // Scroll Y
const SCX_ADDR: u16 = 0xFF43; // Scroll X
const LY_ADDR: u16 = 0xFF44; // LCD Y Coordinate
const LYC_ADDR: u16 = 0xFF45; // LY Compare
const DMA_ADDR: u16 = 0xFF46; // DMA Transfer Register
const BGP_ADDR: u16 = 0xFF47; // BG Palette Data
const OBP0_ADDR: u16 = 0xFF48; // Object Palette 0 Data
const OBP1_ADDR: u16 = 0xFF49; // Object Palette 1 Data
const WY_ADDR: u16 = 0xFF4A; // Window Y Position
const WX_ADDR: u16 = 0xFF4B; // Window X Position

// --- Interrupt Bits (0-4) ---
const VBLANK_INTERRUPT_BIT: u8 = 0;
const LCD_STAT_INTERRUPT_BIT: u8 = 1;
const TIMER_INTERRUPT_BIT: u8 = 2;
const SERIAL_INTERRUPT_BIT: u8 = 3;
const JOYPAD_INTERRUPT_BIT: u8 = 4;

// --- MBC Types ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MbcType {
    NoMbc,
    Mbc1,
    Mbc3,
    // Future: Mbc2, Mbc5, etc.
}

// --- RTC Registers (for MBC3) ---
#[derive(Clone, Debug, Default)]
struct RtcRegisters {
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

    // Basic update based on system time - A real emulator might tie this to internal clock cycles
    fn update(&mut self) {
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
        let mut total_seconds = u64::from(self.seconds) + elapsed_secs;
        self.seconds = (total_seconds % 60) as u8;

        let mut total_minutes = u64::from(self.minutes) + (total_seconds / 60);
        self.minutes = (total_minutes % 60) as u8;

        let mut total_hours = u64::from(self.hours) + (total_minutes / 60);
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

    fn read(&self, reg_select: u8) -> u8 {
        match reg_select {
            0x08 => self.seconds,
            0x09 => self.minutes,
            0x0A => self.hours,
            0x0B => self.dl,
            0x0C => self.dh,
            _ => 0xFF, // Invalid RTC register selection
        }
    }

    fn write(&mut self, reg_select: u8, value: u8) {
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
// --- End RTCRegisters ---

// --- Joypad State ---
#[derive(Clone, Debug, Default)]
pub struct JoypadState {
    // Keep private, accessed via methods
    // True = pressed, False = released (inverted for P1 read)
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
}

/// Represents the Game Boy's memory map with MBC1/MBC3 support and input handling.
#[derive(Clone)]
pub struct MemoryBus {
    // Core Memory Areas
    rom_bank_0: Box<[u8; ROM_BANK_0_SIZE]>,
    vram: Box<[u8; VRAM_SIZE]>,
    wram_bank_0: Box<[u8; WRAM_BANK_0_SIZE]>,
    wram_bank_n: Box<[u8; WRAM_BANK_N_SIZE]>,
    oam: Box<[u8; OAM_SIZE]>,
    io_registers: Box<[u8; IO_REGISTERS_SIZE]>,
    hram: Box<[u8; HRAM_SIZE]>,
    interrupt_enable: u8, // FFFF (IE Register)

    // Cartridge Data & State
    full_rom_data: Vec<u8>,
    external_ram: Vec<u8>,
    mbc_type: MbcType,
    has_ram: bool,
    has_battery: bool, // For saving RAM/RTC state

    // MBC State
    current_rom_bank: usize,
    current_ram_bank: usize, // Also used for RTC register select in MBC3
    ram_enabled: bool,
    banking_mode: u8, // 0=ROM Banking Mode, 1=RAM Banking Mode (MBC1)

    // MBC1 specific intermediate registers
    mbc1_rom_bank_lower: u8,
    mbc1_bank_upper: u8, // RAM bank or ROM bank upper bits

    // MBC3 specific RTC state
    rtc: RtcRegisters,
    rtc_latched: RtcRegisters,
    rtc_latch_state: u8,     // 0: Ready, 1: 0x00 written, 2: 0x01 written (latch)
    rtc_mapped_register: u8, // Which RTC reg (0x08-0x0C) is mapped via RAM bank select

    // Input State
    pub joypad: JoypadState,

    // Calculated sizes (from ROM header)
    num_rom_banks: usize,
    num_ram_banks: usize,
}

impl MemoryBus {
    pub fn new() -> Self {
        // Initialize IO registers with known default values after boot ROM (if skipping)
        // Reference: Pandocs - Power Up Sequence
        let mut io_regs = [0u8; IO_REGISTERS_SIZE];
        io_regs[(P1_JOYP_ADDR - IO_REGISTERS_START) as usize] = 0xCF; // P1 (Joypad) - No selection, reads high
        io_regs[(SC_ADDR - IO_REGISTERS_START) as usize] = 0x7E; // SC (Serial Control)
        io_regs[(TIMA_ADDR - IO_REGISTERS_START) as usize] = 0x00; // TIMA
        io_regs[(TMA_ADDR - IO_REGISTERS_START) as usize] = 0x00; // TMA
        io_regs[(TAC_ADDR - IO_REGISTERS_START) as usize] = 0x00; // TAC
        io_regs[(IF_ADDR - IO_REGISTERS_START) as usize] = 0xE1; // IF - Pandocs says 0xE1 after boot (bits 0,1,2 = VBL,LCD,TIM) - Some emus use 0x01? Check boot ROM behavior. Let's use E1 based on pandocs text.
        io_regs[(NR10_ADDR - IO_REGISTERS_START) as usize] = 0x80; // NR10
        // ... Initialize NR11-NR52 with their defaults ... (0xFF for most initially)
        io_regs[(LCDC_ADDR - IO_REGISTERS_START) as usize] = 0x91; // LCDC - LCD On, BG On, Win Off, TileData#0, BGMap#0, OBJ 8x8, OBJ On
        io_regs[(STAT_ADDR - IO_REGISTERS_START) as usize] = 0x85; // STAT - Mode 1 (VBLANK) + LYC=LY flag set? Check boot ROM. Often 0x80 or 0x81 used. Let's use 0x85 from pandocs table?
        io_regs[(SCY_ADDR - IO_REGISTERS_START) as usize] = 0x00; // SCY
        io_regs[(SCX_ADDR - IO_REGISTERS_START) as usize] = 0x00; // SCX
        io_regs[(LY_ADDR - IO_REGISTERS_START) as usize] = 0x00; // LY - Set to 0 initially? PPU will drive this.
        io_regs[(LYC_ADDR - IO_REGISTERS_START) as usize] = 0x00; // LYC
        io_regs[(BGP_ADDR - IO_REGISTERS_START) as usize] = 0xFC; // BGP Palette
        io_regs[(OBP0_ADDR - IO_REGISTERS_START) as usize] = 0xFF; // OBP0 Palette
        io_regs[(OBP1_ADDR - IO_REGISTERS_START) as usize] = 0xFF; // OBP1 Palette
        io_regs[(WY_ADDR - IO_REGISTERS_START) as usize] = 0x00; // WY
        io_regs[(WX_ADDR - IO_REGISTERS_START) as usize] = 0x00; // WX

        MemoryBus {
            rom_bank_0: Box::new([0; ROM_BANK_0_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            wram_bank_0: Box::new([0; WRAM_BANK_0_SIZE]),
            wram_bank_n: Box::new([0; WRAM_BANK_N_SIZE]), // DMG only uses Bank 1 effectively
            oam: Box::new([0; OAM_SIZE]),
            io_registers: Box::new(io_regs), // Use initialized IO regs
            hram: Box::new([0; HRAM_SIZE]),
            interrupt_enable: 0x00, // IE register starts at 0x00

            full_rom_data: Vec::new(),
            external_ram: Vec::new(),
            mbc_type: MbcType::NoMbc,
            has_ram: false,
            has_battery: false,

            current_rom_bank: 1, // Default for banks 1-N
            current_ram_bank: 0,
            ram_enabled: false,
            banking_mode: 0,

            mbc1_rom_bank_lower: 1,
            mbc1_bank_upper: 0,

            rtc: RtcRegisters::default(),
            rtc_latched: RtcRegisters::default(),
            rtc_latch_state: 0,
            rtc_mapped_register: 0,

            joypad: JoypadState::default(), // All buttons released initially

            num_rom_banks: 2, // Default (e.g., for 32KB ROM)
            num_ram_banks: 0,
        }
    }

    /// Loads ROM data and configures MBC based on the header.
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        if rom_data.len() < 0x150 {
            // Need at least header size
            panic!("ROM data is too small to contain a valid header");
        }

        // Basic header info
        let cartridge_type_code = rom_data[0x0147];
        let rom_size_code = rom_data[0x0148];
        let ram_size_code = rom_data[0x0149];

        // Determine MBC Type, RAM, Battery
        (self.mbc_type, self.has_ram, self.has_battery) = match cartridge_type_code {
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
        };

        // Determine ROM size and number of banks
        self.num_rom_banks = match rom_size_code {
            0x00 => 2,
            0x01 => 4,
            0x02 => 8,
            0x03 => 16,
            0x04 => 32,
            0x05 => 64,
            0x06 => 128,
            0x07 => 256,
            0x08 => 512,
            0x52 => 72,
            0x53 => 80,
            0x54 => 96, // Less common sizes
            _ => panic!("Unsupported ROM size code: {:02X}", rom_size_code),
        };
        let expected_rom_size = self.num_rom_banks * ROM_BANK_N_SIZE;
        if rom_data.len() < expected_rom_size {
            println!(
                "Warning: ROM file size ({}) is smaller than expected ({}) based on header.",
                rom_data.len(),
                expected_rom_size
            );
            // Adjust num_rom_banks or just be careful with reads? Let's keep header value for now.
        }
        if rom_data.len() > expected_rom_size {
            println!(
                "Warning: ROM file size ({}) is larger than expected ({}) based on header. Extra data might be ignored by banking.",
                rom_data.len(),
                expected_rom_size
            );
        }

        // Determine RAM size and number of banks
        let ram_size = match ram_size_code {
            0x00 | 0x01 => 0,   // No RAM
            0x02 => 8 * 1024,   // 8 KiB (1 bank)
            0x03 => 32 * 1024,  // 32 KiB (4 banks)
            0x04 => 128 * 1024, // 128 KiB (16 banks) - More common for MBC5
            0x05 => 64 * 1024,  // 64 KiB (8 banks)
            _ => panic!("Unsupported RAM size code: {:02X}", ram_size_code),
        };

        // Consistency checks between type and RAM size
        if ram_size > 0 && !self.has_ram {
            println!(
                "Warning: Cartridge header RAM size {:02X} but type {:02X} usually lacks RAM.",
                ram_size_code, cartridge_type_code
            );
        }
        if ram_size == 0 && self.has_ram && self.mbc_type != MbcType::Mbc3 {
            // MBC3 might have only Timer/Batt
            println!(
                "Warning: Cartridge header RAM size 00 but type {:02X} expects RAM.",
                cartridge_type_code
            );
        }

        // Store ROM data
        self.full_rom_data = rom_data.to_vec();
        if self.full_rom_data.len() >= ROM_BANK_0_SIZE {
            self.rom_bank_0
                .copy_from_slice(&self.full_rom_data[0..ROM_BANK_0_SIZE]);
        } else {
            // ROM is smaller than bank 0 size? Very unusual. Pad? Panic?
            panic!("ROM is smaller than 16KB, cannot load into Bank 0.");
        }

        // Initialize External RAM
        if self.has_ram && ram_size > 0 {
            self.external_ram = vec![0u8; ram_size]; // Initialize with zeros
            self.num_ram_banks = ram_size / EXT_RAM_SIZE;
            if self.num_ram_banks == 0 && ram_size > 0 {
                self.num_ram_banks = 1;
            } // Handle 2KB, 8KB cases etc.
        } else {
            self.has_ram = false; // Ensure consistency
            self.external_ram = Vec::new();
            self.num_ram_banks = 0;
        }

        // Reset MBC state variables to defaults
        self.current_rom_bank = 1;
        self.current_ram_bank = 0;
        self.ram_enabled = false;
        self.banking_mode = 0;
        self.mbc1_rom_bank_lower = 1;
        self.mbc1_bank_upper = 0;
        self.rtc = RtcRegisters::default();
        self.rtc_latched = RtcRegisters::default();
        self.rtc_latch_state = 0;
        self.rtc_mapped_register = 0; // 0 indicates RAM bank 0 selected

        println!(
            "Loaded ROM: {} bytes. Type: {:?} ({:02X}), ROM Banks: {}, RAM Banks: {} ({} KB), Battery: {}",
            self.full_rom_data.len(),
            self.mbc_type,
            cartridge_type_code,
            self.num_rom_banks,
            self.num_ram_banks,
            ram_size / 1024,
            self.has_battery
        );
    }

    // --- MBC Helper Logic ---

    /// Updates the effective ROM bank for MBC1 based on current register values.
    fn update_mbc1_rom_bank(&mut self) {
        let mut bank = self.mbc1_rom_bank_lower as usize;
        if self.banking_mode == 0 {
            // ROM mode: combine lower 5 bits with upper 2 bits from 0x4000-0x5FFF writes
            bank |= (self.mbc1_bank_upper as usize) << 5;
        }
        // Bank 0 is never selected this way; writing 0 selects 1.
        // Also handles banks like 0x20, 0x40, 0x60 which are aliases of 0x21, 0x41, 0x61
        if bank == 0 || bank == 0x20 || bank == 0x40 || bank == 0x60 {
            bank += 1;
        }

        // Mask the bank number to the actual number of banks available
        self.current_rom_bank = bank & (self.num_rom_banks - 1);
    }

    /// Updates the effective RAM bank for MBC1 based on current register values.
    fn update_mbc1_ram_bank(&mut self) {
        if self.banking_mode == 1 {
            // RAM mode: Use upper 2 bits from 0x4000-0x5FFF writes directly
            self.current_ram_bank = self.mbc1_bank_upper as usize;
        } else {
            // ROM mode: RAM bank is always 0
            self.current_ram_bank = 0;
        }
        // Mask to available RAM banks
        if self.num_ram_banks > 0 {
            self.current_ram_bank &= (self.num_ram_banks - 1);
        } else {
            self.current_ram_bank = 0; // Safety if no RAM banks exist
        }
    }

    // --- Interrupt Request Helper ---
    /// Sets the corresponding interrupt flag bit (0-4) in the IF register (0xFF0F).
    pub fn request_interrupt(&mut self, bit: u8) {
        if bit < 5 {
            let if_reg_offset = (IF_ADDR - IO_REGISTERS_START) as usize;
            let current_if = self.io_registers[if_reg_offset];
            self.io_registers[if_reg_offset] = current_if | (1 << bit);
            // println!("Interrupt Requested: Bit {}", bit); // Debug
        }
    }

    // --- Read/Write ---

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // ROM Bank 0 (Fixed)
            ROM_BANK_0_START..=ROM_BANK_0_END => self.rom_bank_0[addr as usize],
            // ROM Bank N (Switchable)
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                if self.current_rom_bank >= self.num_rom_banks {
                    // Attempt to read from a bank beyond the cartridge's size
                    return 0xFF; // Open bus behavior? Typically reads FF.
                }
                let rom_offset =
                    (self.current_rom_bank * ROM_BANK_N_SIZE) + (addr - ROM_BANK_N_START) as usize;
                if rom_offset < self.full_rom_data.len() {
                    self.full_rom_data[rom_offset]
                } else {
                    // Access beyond the actual ROM data size (e.g. if file was smaller than header indicated)
                    0xFF
                }
            }
            // Video RAM (VRAM)
            VRAM_START..=VRAM_END => {
                // TODO: Proper PPU mode checking (VRAM accessible during Mode 0, 1, 2)
                // For now, allow read. PPU struct should manage restrictions ideally.
                self.vram[(addr - VRAM_START) as usize]
            }
            // External RAM / RTC Registers
            EXT_RAM_START..=EXT_RAM_END => {
                if !self.ram_enabled {
                    return 0xFF; // Access disabled
                }
                match self.mbc_type {
                    MbcType::Mbc3
                        if self.rtc_mapped_register >= 0x08 && self.rtc_mapped_register <= 0x0C =>
                    {
                        self.rtc_latched.read(self.rtc_mapped_register)
                    }
                    _ => {
                        // Includes NoMbc RAM, Mbc1 RAM, and Mbc3 RAM access
                        if !self.has_ram || self.external_ram.is_empty() {
                            return 0xFF;
                        }
                        if self.current_ram_bank >= self.num_ram_banks {
                            return 0xFF;
                        }

                        let ram_offset = (self.current_ram_bank * EXT_RAM_SIZE)
                            + (addr - EXT_RAM_START) as usize;
                        if ram_offset < self.external_ram.len() {
                            self.external_ram[ram_offset]
                        } else {
                            0xFF
                        } // Should not happen if bounds checks above are correct
                    }
                }
            }
            // Work RAM Bank 0 (Always accessible)
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize]
            }
            // Work RAM Bank N (Fixed Bank 1 on DMG)
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize]
            }
            // Echo RAM (Mirrors C000-DDFF)
            ECHO_RAM_START..=ECHO_RAM_END => self.read_byte(addr - 0x2000),
            // OAM (Object Attribute Memory)
            OAM_START..=OAM_END => {
                // TODO: Proper PPU mode checking (OAM accessible during Mode 0, 1)
                // PPU struct should manage restrictions. Allow read for now.
                self.oam[(addr - OAM_START) as usize]
            }
            // Not Usable Area
            NOT_USABLE_START..=NOT_USABLE_END => 0xFF,
            // I/O Registers
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                let offset = (addr - IO_REGISTERS_START) as usize;
                match addr {
                    P1_JOYP_ADDR => {
                        // Joypad Read
                        let selection_bits = self.io_registers[offset] & 0x30; // Read bits 4,5 written by game
                        let mut joypad_value = 0x0F; // Start with lower nibble high (released)

                        if selection_bits & 0x20 == 0 {
                            // Bit 5 Low: Select Action buttons
                            if self.joypad.a {
                                joypad_value &= 0b1110;
                            } // Bit 0 low if pressed
                            if self.joypad.b {
                                joypad_value &= 0b1101;
                            } // Bit 1 low if pressed
                            if self.joypad.select {
                                joypad_value &= 0b1011;
                            } // Bit 2 low if pressed
                            if self.joypad.start {
                                joypad_value &= 0b0111;
                            } // Bit 3 low if pressed
                        }
                        if selection_bits & 0x10 == 0 {
                            // Bit 4 Low: Select Direction buttons
                            if self.joypad.right {
                                joypad_value &= 0b1110;
                            } // Bit 0 low if pressed
                            if self.joypad.left {
                                joypad_value &= 0b1101;
                            } // Bit 1 low if pressed
                            if self.joypad.up {
                                joypad_value &= 0b1011;
                            } // Bit 2 low if pressed
                            if self.joypad.down {
                                joypad_value &= 0b0111;
                            } // Bit 3 low if pressed
                        }
                        // Combine input bits (0-3) with selection bits (4-5) and unused high bits (reads 1)
                        joypad_value | selection_bits | 0xC0
                    }
                    // Add reads for other registers that have side effects or specific behavior
                    // e.g., Reading STAT might depend on current PPU state
                    STAT_ADDR => {
                        // Combine PPU state (Mode, LYC flag) with writable bits (Interrupt enables)
                        // This should ideally be updated by the PPU itself writing to STAT
                        // Reading here just returns the last value written + PPU updates.
                        self.io_registers[offset] | 0x80 // Bit 7 always reads high
                    }
                    LY_ADDR => {
                        // Should be written by PPU. Reading here gives the current value.
                        self.io_registers[offset]
                    }
                    _ => self.io_registers[offset], // Default read for other registers
                }
            }
            // High RAM (HRAM)
            HRAM_START..=HRAM_END => self.hram[(addr - HRAM_START) as usize],
            // Interrupt Enable Register (IE)
            INTERRUPT_ENABLE_REGISTER => self.interrupt_enable,
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            // --- MBC Control Registers ---
            0x0000..=0x1FFF => {
                // RAM/RTC Enable (MBC1, MBC3)
                match self.mbc_type {
                    MbcType::Mbc1 | MbcType::Mbc3 => {
                        if self.has_ram || self.mbc_type == MbcType::Mbc3 {
                            // MBC3 always has RTC chip
                            self.ram_enabled = (value & 0x0F) == 0x0A;
                        }
                    }
                    _ => {} // No effect for NoMbc
                }
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number (Lower bits)
                match self.mbc_type {
                    MbcType::Mbc1 => {
                        let bank_low = value & 0x1F; // Lower 5 bits
                        self.mbc1_rom_bank_lower = if bank_low == 0 { 1 } else { bank_low }; // Bank 0 maps to 1
                        self.update_mbc1_rom_bank();
                    }
                    MbcType::Mbc3 => {
                        let bank = value & 0x7F; // 7 bits
                        self.current_rom_bank = if bank == 0 { 1 } else { bank as usize };
                        self.current_rom_bank &= (self.num_rom_banks - 1); // Mask to available banks
                    }
                    _ => {} // No effect for NoMbc
                }
            }
            0x4000..=0x5FFF => {
                // RAM Bank Number / ROM Bank Upper Bits (MBC1) / RTC Register Select (MBC3)
                match self.mbc_type {
                    MbcType::Mbc1 => {
                        self.mbc1_bank_upper = value & 0x03; // 2 bits used
                        self.update_mbc1_rom_bank(); // ROM bank might change if in mode 0
                        self.update_mbc1_ram_bank(); // RAM bank might change if in mode 1
                    }
                    MbcType::Mbc3 => {
                        if value <= 0x07 {
                            // Select RAM Bank 0-7
                            self.current_ram_bank = value as usize;
                            self.rtc_mapped_register = 0; // Indicate RAM is selected
                            // Mask to available banks
                            if self.num_ram_banks > 0 {
                                self.current_ram_bank &= (self.num_ram_banks - 1);
                            } else {
                                self.current_ram_bank = 0; // No RAM banks available
                            }
                        } else if value >= 0x08 && value <= 0x0C {
                            // Select RTC Register
                            self.rtc_mapped_register = value;
                        } else { /* Invalid value ignored */
                        }
                    }
                    _ => {} // No effect for NoMbc
                }
            }
            0x6000..=0x7FFF => {
                // Banking Mode Select (MBC1) / Latch Clock Data (MBC3)
                match self.mbc_type {
                    MbcType::Mbc1 => {
                        self.banking_mode = value & 0x01;
                        self.update_mbc1_rom_bank(); // Re-calculate banks based on new mode
                        self.update_mbc1_ram_bank();
                    }
                    MbcType::Mbc3 => {
                        if self.rtc_latch_state == 0 && value == 0x00 {
                            self.rtc_latch_state = 1; // Got 0x00
                        } else if self.rtc_latch_state == 1 && value == 0x01 {
                            // Got 0x01 after 0x00, perform latch
                            self.rtc.update(); // Ensure RTC state is current before latching
                            self.rtc_latched = self.rtc.clone();
                            self.rtc_latch_state = 0; // Reset for next latch sequence
                        } else {
                            self.rtc_latch_state = 0; // Invalid sequence, reset
                        }
                    }
                    _ => {} // No effect for NoMbc
                }
            }

            // --- Normal Memory Areas ---
            // Video RAM (VRAM)
            VRAM_START..=VRAM_END => {
                // TODO: Proper PPU mode checking (VRAM write only allowed during Mode 0, 1, 2?)
                self.vram[(addr - VRAM_START) as usize] = value;
            }
            // External RAM / RTC Registers
            EXT_RAM_START..=EXT_RAM_END => {
                if !self.ram_enabled {
                    return;
                } // Write ignored if disabled
                match self.mbc_type {
                    MbcType::Mbc3
                        if self.rtc_mapped_register >= 0x08 && self.rtc_mapped_register <= 0x0C =>
                    {
                        // Writing to live RTC register (not latched version)
                        self.rtc.write(self.rtc_mapped_register, value);
                    }
                    _ => {
                        // RAM access
                        if !self.has_ram || self.external_ram.is_empty() {
                            return;
                        }
                        if self.current_ram_bank >= self.num_ram_banks {
                            return;
                        }
                        let ram_offset = (self.current_ram_bank * EXT_RAM_SIZE)
                            + (addr - EXT_RAM_START) as usize;
                        if ram_offset < self.external_ram.len() {
                            self.external_ram[ram_offset] = value;
                        }
                    }
                }
            }
            // Work RAM Bank 0
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize] = value;
            }
            // Work RAM Bank N
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize] = value;
            }
            // Echo RAM
            ECHO_RAM_START..=ECHO_RAM_END => {
                self.write_byte(addr - 0x2000, value);
            }
            // OAM
            OAM_START..=OAM_END => {
                // TODO: Proper PPU mode checking (OAM write only allowed during Mode 0, 1?)
                self.oam[(addr - OAM_START) as usize] = value;
            }
            // Not Usable Area
            NOT_USABLE_START..=NOT_USABLE_END => { /* Write Ignored */ }
            // I/O Registers
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                let offset = (addr - IO_REGISTERS_START) as usize;
                match addr {
                    P1_JOYP_ADDR => {
                        // Joypad Write (only bits 4, 5 are writable)
                        self.io_registers[offset] =
                            (self.io_registers[offset] & 0xCF) | (value & 0x30);
                    }
                    DIV_ADDR => {
                        // DIV - Writing any value resets it to 0
                        self.io_registers[offset] = 0;
                        // TODO: Reset internal timer divider counter in Timer component
                    }
                    TIMA_ADDR | TMA_ADDR | TAC_ADDR => {
                        self.io_registers[offset] = value;
                        // TODO: Notify Timer component of potential change
                    }
                    IF_ADDR => {
                        // Interrupt Flags - Writing 1 clears the flag
                        self.io_registers[offset] &= !value;
                    }
                    LCDC_ADDR | STAT_ADDR | SCY_ADDR | SCX_ADDR | LYC_ADDR | BGP_ADDR
                    | OBP0_ADDR | OBP1_ADDR | WY_ADDR | WX_ADDR => {
                        self.io_registers[offset] = value;
                        // TODO: Notify PPU component of potential change
                    }
                    DMA_ADDR => {
                        // DMA Transfer Trigger
                        self.io_registers[offset] = value;
                        self.perform_dma_transfer(value);
                    }
                    // TODO: Add Sound Register writes (FF10-FF26, FF30-FF3F)
                    _ => {
                        // Default write for unhandled IO regs
                        self.io_registers[offset] = value;
                    }
                }
            }
            // High RAM (HRAM)
            HRAM_START..=HRAM_END => {
                self.hram[(addr - HRAM_START) as usize] = value;
            }
            // Interrupt Enable Register (IE)
            INTERRUPT_ENABLE_REGISTER => {
                self.interrupt_enable = value;
            }
        }
    }

    // --- Helper methods ---

    /// Performs an OAM DMA transfer.
    fn perform_dma_transfer(&mut self, source_high_byte: u8) {
        // Source address is 0xXX00 where XX is the value written (e.g., C0 for 0xC000)
        let source_start_addr = (source_high_byte as u16) << 8;
        // DMA typically can't read from certain areas like OAM itself or HRAM above FFDF?
        // Source is usually ROM or WRAM. Assume valid source for now.

        for i in 0..OAM_SIZE {
            // Copy 160 bytes
            let byte_to_copy = self.read_byte(source_start_addr + i as u16);
            // Write directly to OAM memory. PPU OAM restrictions might be ignored during DMA.
            self.oam[i] = byte_to_copy;
        }
        // IMPORTANT TODO: This transfer takes ~160 machine cycles (~640 T-cycles).
        // The CPU should be paused during this time. This needs to be handled
        // by the main emulator loop or CPU stepping logic, potentially by
        // returning a special value or setting a flag in the bus.
    }

    /// Reads a 16-bit word (Little Endian) - Optional helper
    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    /// Writes a 16-bit word (Little Endian) - Optional helper
    pub fn write_word(&mut self, addr: u16, value: u16) {
        let low = (value & 0xFF) as u8;
        let high = (value >> 8) as u8;
        self.write_byte(addr, low);
        self.write_byte(addr.wrapping_add(1), high);
    }

    /// Called periodically (e.g., per frame or more often) to update RTC state.
    pub fn tick_rtc(&mut self) {
        if self.mbc_type == MbcType::Mbc3 {
            self.rtc.update();
        }
    }

    // --- Public Input Handling Methods ---

    /// Called by the frontend when a key mapped to a Game Boy button is pressed down.
    pub fn key_down(&mut self, key: Keycode) {
        let p1_offset = (P1_JOYP_ADDR - IO_REGISTERS_START) as usize;
        let p1_reg = self.io_registers[p1_offset]; // Read current P1 selection state
        let mut button_newly_pressed = false;
        let mut selection_active = false;

        match key {
            // Directions (Check bit 4 of P1 register)
            Keycode::Right | Keycode::D => {
                if !self.joypad.right {
                    button_newly_pressed = true;
                    self.joypad.right = true;
                }
                if p1_reg & 0x10 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Left | Keycode::A => {
                // Remap 'A' key to Left
                if !self.joypad.left {
                    button_newly_pressed = true;
                    self.joypad.left = true;
                }
                if p1_reg & 0x10 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Up | Keycode::W => {
                if !self.joypad.up {
                    button_newly_pressed = true;
                    self.joypad.up = true;
                }
                if p1_reg & 0x10 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Down | Keycode::S => {
                if !self.joypad.down {
                    button_newly_pressed = true;
                    self.joypad.down = true;
                }
                if p1_reg & 0x10 == 0 {
                    selection_active = true;
                }
            }
            // Actions (Check bit 5 of P1 register)
            Keycode::Z | Keycode::J => {
                // GB 'A' button
                if !self.joypad.a {
                    button_newly_pressed = true;
                    self.joypad.a = true;
                }
                if p1_reg & 0x20 == 0 {
                    selection_active = true;
                }
            }
            Keycode::X | Keycode::K => {
                // GB 'B' button
                if !self.joypad.b {
                    button_newly_pressed = true;
                    self.joypad.b = true;
                }
                if p1_reg & 0x20 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Backspace | Keycode::RShift => {
                // GB 'Select' button
                if !self.joypad.select {
                    button_newly_pressed = true;
                    self.joypad.select = true;
                }
                if p1_reg & 0x20 == 0 {
                    selection_active = true;
                }
            }
            Keycode::Return | Keycode::Space => {
                // GB 'Start' button
                if !self.joypad.start {
                    button_newly_pressed = true;
                    self.joypad.start = true;
                }
                if p1_reg & 0x20 == 0 {
                    selection_active = true;
                }
            }
            _ => {} // Ignore other keys
        }

        // Request Joypad interrupt only if a button state changed from released->pressed
        // AND that button's group (Directions/Actions) is currently selected by the game.
        if button_newly_pressed && selection_active {
            self.request_interrupt(JOYPAD_INTERRUPT_BIT);
        }
    }

    /// Called by the frontend when a key mapped to a Game Boy button is released.
    pub fn key_up(&mut self, key: Keycode) {
        match key {
            Keycode::Right | Keycode::D => self.joypad.right = false,
            Keycode::Left | Keycode::A => self.joypad.left = false,
            Keycode::Up | Keycode::W => self.joypad.up = false,
            Keycode::Down | Keycode::S => self.joypad.down = false,
            Keycode::Z | Keycode::J => self.joypad.a = false,
            Keycode::X | Keycode::K => self.joypad.b = false,
            Keycode::Backspace | Keycode::RShift => self.joypad.select = false,
            Keycode::Return | Keycode::Space => self.joypad.start = false,
            _ => {} // Ignore other keys
        }
    }
} // impl MemoryBus

// Implement Debug for easier printing/logging
impl fmt::Debug for MemoryBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryBus")
            .field("mbc_type", &self.mbc_type)
            .field("rom_banks", &self.num_rom_banks)
            .field("ram_banks", &self.num_ram_banks)
            .field("has_ram", &self.has_ram)
            .field("has_battery", &self.has_battery)
            .field("ram_enabled", &self.ram_enabled)
            .field("current_rom_bank", &self.current_rom_bank)
            .field("current_ram_bank", &self.current_ram_bank)
            .field("banking_mode(MBC1)", &self.banking_mode)
            .field("rtc_mapped(MBC3)", &self.rtc_mapped_register)
            .field(
                "interrupt_enable",
                &format_args!("{:#04X}", self.interrupt_enable),
            )
            .field("joypad_state", &self.joypad) // Show current button state
            // Avoid printing large arrays by showing size or omitting
            // .field("vram", &self.vram) // Too large
            // .field("external_ram", &self.external_ram) // Too large
            .finish_non_exhaustive() // Indicates other fields exist but aren't shown
    }
}
