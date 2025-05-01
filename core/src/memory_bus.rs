use std::{fmt, time::{SystemTime, UNIX_EPOCH}}; // Added SystemTime for basic RTC

// --- Memory Map Constants (Unchanged) ---
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

// --- MBC Types ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MbcType {
    NoMbc,
    Mbc1,
    Mbc3,
    // Mbc2, Mbc5, etc. can be added here
}

// --- RTC Registers (for MBC3) ---
#[derive(Clone, Debug, Default)]
struct RtcRegisters {
    seconds: u8, // 0x08
    minutes: u8, // 0x09
    hours: u8,   // 0x0A
    dl: u8,      // 0x0B (Lower 8 bits of day counter)
    dh: u8,      // 0x0C (Upper 1 bit of day counter + flags)

    // Internal state for timing
    last_updated_secs: u64,
}

impl RtcRegisters {
    const DAY_CARRY_BIT: u8 = 0b0000_0001; // Bit 0 in DH
    const HALT_BIT: u8 = 0b0100_0000;      // Bit 6 in DH
    const DAY_OVERFLOW_BIT: u8 = 0b1000_0000; // Bit 7 in DH (Read only)

    // Basic update based on system time - replace with emulator cycle counting if needed
    fn update(&mut self) {
        if (self.dh & RtcRegisters::HALT_BIT) != 0 {
             self.last_updated_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            return; // Timer halted
        }

        let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let elapsed_secs = now_secs.saturating_sub(self.last_updated_secs);

        if elapsed_secs == 0 {
            return;
        }

        self.last_updated_secs = now_secs;

        let mut total_seconds = u64::from(self.seconds) + elapsed_secs;
        self.seconds = (total_seconds % 60) as u8;

        let mut total_minutes = u64::from(self.minutes) + (total_seconds / 60);
        self.minutes = (total_minutes % 60) as u8;

        let mut total_hours = u64::from(self.hours) + (total_minutes / 60);
        self.hours = (total_hours % 24) as u8;

        let mut days = u64::from(self.dl) | (u64::from(self.dh & RtcRegisters::DAY_CARRY_BIT) << 8);
        days += total_hours / 24;

        if days >= 512 {
            self.dh |= RtcRegisters::DAY_OVERFLOW_BIT; // Set overflow
            days %= 512; // Wrap around
        }

        self.dl = (days & 0xFF) as u8;
        self.dh = (self.dh & !RtcRegisters::DAY_CARRY_BIT) | ((days >> 8) & RtcRegisters::DAY_CARRY_BIT as u64) as u8;
    }

    fn read(&self, reg_select: u8) -> u8 {
        match reg_select {
            0x08 => self.seconds,
            0x09 => self.minutes,
            0x0A => self.hours,
            0x0B => self.dl,
            0x0C => self.dh,
            _ => 0xFF, // Should not happen if select logic is correct
        }
    }

    fn write(&mut self, reg_select: u8, value: u8) {
        match reg_select {
            0x08 => self.seconds = value % 60, // Clamp values
            0x09 => self.minutes = value % 60,
            0x0A => self.hours = value % 24,
            0x0B => self.dl = value,
            0x0C => self.dh = value & 0b1100_0001, // Mask RW bits (Halt, Day Carry)
            _ => {}, // Should not happen
        }
    }
}

/// Represents the Game Boy's memory map with MBC1/MBC3 support.
#[derive(Clone)]
pub struct MemoryBus {
    // Core Memory Areas
    rom_bank_0: Box<[u8; ROM_BANK_0_SIZE]>, // 0000-3FFF (Always Bank 0)
    vram: Box<[u8; VRAM_SIZE]>,          // 8000-9FFF
    wram_bank_0: Box<[u8; WRAM_BANK_0_SIZE]>, // C000-CFFF
    wram_bank_n: Box<[u8; WRAM_BANK_N_SIZE]>, // D000-DFFF (Bank 1, CGB would switch this)
    oam: Box<[u8; OAM_SIZE]>,             // FE00-FE9F
    io_registers: Box<[u8; IO_REGISTERS_SIZE]>, // FF00-FF7F
    hram: Box<[u8; HRAM_SIZE]>,           // FF80-FFFE
    interrupt_enable: u8,                 // FFFF

    // Cartridge Data & State
    full_rom_data: Vec<u8>,               // Complete ROM
    external_ram: Vec<u8>,                // Cartridge RAM (if present)
    mbc_type: MbcType,
    has_ram: bool,
    has_battery: bool, // For saving RAM/RTC state later

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
    rtc_latch_state: u8, // 0: Ready, 1: 0x00 written, 2: 0x01 written (latched)
    rtc_mapped_register: u8, // Which RTC reg (0x08-0x0C) is mapped via RAM bank select

    // Calculated sizes
    num_rom_banks: usize,
    num_ram_banks: usize,
}

impl MemoryBus {
    pub fn new() -> Self {
        MemoryBus {
            rom_bank_0: Box::new([0; ROM_BANK_0_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            wram_bank_0: Box::new([0; WRAM_BANK_0_SIZE]),
            wram_bank_n: Box::new([0; WRAM_BANK_N_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            io_registers: Box::new([0; IO_REGISTERS_SIZE]), // TODO: Initialize properly
            hram: Box::new([0; HRAM_SIZE]),
            interrupt_enable: 0,

            full_rom_data: Vec::new(),
            external_ram: Vec::new(),
            mbc_type: MbcType::NoMbc,
            has_ram: false,
            has_battery: false,

            current_rom_bank: 1, // Default to bank 1
            current_ram_bank: 0,
            ram_enabled: false,
            banking_mode: 0,

            mbc1_rom_bank_lower: 1,
            mbc1_bank_upper: 0,

            rtc: RtcRegisters::default(),
            rtc_latched: RtcRegisters::default(),
            rtc_latch_state: 0,
            rtc_mapped_register: 0, // 0 indicates RAM bank 0 selected

            num_rom_banks: 2, // Default for 32KB ROM
            num_ram_banks: 0,
        }
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) {
        if rom_data.len() < ROM_BANK_0_SIZE {
            panic!("ROM data is too small (less than 16KB)");
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
            // 0x05 => (MbcType::Mbc2, false, false), // MBC2 Not implemented
            // 0x06 => (MbcType::Mbc2, false, true),  // MBC2 Not implemented
            0x08 => (MbcType::NoMbc, true, false), // ROM+RAM
            0x09 => (MbcType::NoMbc, true, true),  // ROM+RAM+BATT
            0x0F => (MbcType::Mbc3, false, true), // MBC3+TIMER+BATT
            0x10 => (MbcType::Mbc3, true, true),  // MBC3+TIMER+RAM+BATT
            0x11 => (MbcType::Mbc3, false, false), // MBC3
            0x12 => (MbcType::Mbc3, true, false),  // MBC3+RAM
            0x13 => (MbcType::Mbc3, true, true),   // MBC3+RAM+BATT
            // Add other types (MBC2, MBC5 etc.) here
            _ => panic!("Unsupported cartridge type: {:02X}", cartridge_type_code),
        };

        // Determine ROM size
        self.num_rom_banks = match rom_size_code {
            0x00 => 2,   // 32KB (2 banks)
            0x01 => 4,   // 64KB
            0x02 => 8,   // 128KB
            0x03 => 16,  // 256KB
            0x04 => 32,  // 512KB
            0x05 => 64,  // 1MB
            0x06 => 128, // 2MB
            0x07 => 256, // 4MB
            0x08 => 512, // 8MB
            // Unofficial sizes - treat as max?
            0x52 => 72,
            0x53 => 80,
            0x54 => 96,
            _ => panic!("Unsupported ROM size code: {:02X}", rom_size_code),
        };
        let expected_rom_size = self.num_rom_banks * ROM_BANK_N_SIZE; // 16KB per bank
        if rom_data.len() < expected_rom_size {
             // Allow smaller files for header reading? Or just panic?
             println!("Warning: ROM file size ({}) is smaller than expected size ({}) based on header.", rom_data.len(), expected_rom_size);
             // Adjust num_rom_banks? Or just proceed carefully? Let's proceed for now.
             // self.num_rom_banks = rom_data.len() / ROM_BANK_N_SIZE;
        }
         if rom_data.len() > expected_rom_size {
             println!("Warning: ROM file size ({}) is larger than expected size ({}) based on header. Extra data ignored.", rom_data.len(), expected_rom_size);
         }


        // Determine RAM size
        let ram_size = match ram_size_code {
            0x00 => 0,             // No RAM
            0x01 => 0,             // Unused? Often means no RAM.
            0x02 => 8 * 1024,      // 8 KiB (1 bank)
            0x03 => 32 * 1024,     // 32 KiB (4 banks)
            0x04 => 128 * 1024,    // 128 KiB (16 banks)
            0x05 => 64 * 1024,     // 64 KiB (8 banks)
            _ => panic!("Unsupported RAM size code: {:02X}", ram_size_code),
        };

        if ram_size > 0 && !self.has_ram {
            println!("Warning: Cartridge header indicates RAM size {:02X} but type {:02X} doesn't usually have RAM.", ram_size_code, cartridge_type_code);
            // Decide: trust the type (no RAM) or trust the size code? Let's trust the type for now.
            // self.has_ram = true;
        }
         if ram_size == 0 && self.has_ram {
             println!("Warning: Cartridge header indicates RAM size 00 but type {:02X} expects RAM.", cartridge_type_code);
             // Might be MBC3+Timer only case? Keep has_ram true if type implies it.
         }


        // Store ROM
        self.full_rom_data = rom_data.to_vec();
        self.rom_bank_0.copy_from_slice(&self.full_rom_data[0..ROM_BANK_0_SIZE]);

        // Initialize External RAM
        if self.has_ram && ram_size > 0 {
            self.external_ram = vec![0; ram_size];
            self.num_ram_banks = ram_size / EXT_RAM_SIZE;
            if self.num_ram_banks == 0 && ram_size > 0 { self.num_ram_banks = 1;} // Case for 2KB RAM
        } else {
             self.has_ram = false; // Ensure consistency if size was 0
             self.external_ram = Vec::new();
             self.num_ram_banks = 0;
        }


        // Reset MBC state
        self.current_rom_bank = 1;
        self.current_ram_bank = 0;
        self.ram_enabled = false;
        self.banking_mode = 0;
        self.mbc1_rom_bank_lower = 1;
        self.mbc1_bank_upper = 0;
        self.rtc = RtcRegisters::default();
        self.rtc_latched = RtcRegisters::default();
        self.rtc_latch_state = 0;
        self.rtc_mapped_register = 0;


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

    fn update_mbc1_rom_bank(&mut self) {
        let mut bank = self.mbc1_rom_bank_lower as usize;
        if self.banking_mode == 0 {
            // ROM mode: combine lower 5 bits with upper 2 bits
            bank |= (self.mbc1_bank_upper as usize) << 5;
        }
         // Bank 0 is never selected here, writing 0 selects 1
        if bank & (self.num_rom_banks -1) == 0 {
            bank +=1;
        }

        self.current_rom_bank = bank & (self.num_rom_banks - 1);
        // println!("MBC1 ROM Bank Set: {} (Lower: {}, Upper: {}, Mode: {})", self.current_rom_bank, self.mbc1_rom_bank_lower, self.mbc1_bank_upper, self.banking_mode);
    }

     fn update_mbc1_ram_bank(&mut self) {
         if self.banking_mode == 1 {
            // RAM mode: Use upper 2 bits directly
            self.current_ram_bank = self.mbc1_bank_upper as usize;
         } else {
            // ROM mode: RAM bank is always 0
            self.current_ram_bank = 0;
         }
          self.current_ram_bank &= (self.num_ram_banks -1); // Mask if num_ram_banks is power of 2
           if self.num_ram_banks > 0 && self.current_ram_bank >= self.num_ram_banks {
               self.current_ram_bank %= self.num_ram_banks; // Handle non-power-of-2 banks? Seems unlikely for MBC1
           }
        //  println!("MBC1 RAM Bank Set: {} (Upper: {}, Mode: {})", self.current_ram_bank, self.mbc1_bank_upper, self.banking_mode);

     }

    // --- Read/Write ---

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // ROM Bank 0 (Fixed)
            ROM_BANK_0_START..=ROM_BANK_0_END => {
                self.rom_bank_0[addr as usize]
            }
            // ROM Bank N (Switchable)
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                 // Ensure bank number is valid before calculating offset
                 if self.current_rom_bank >= self.num_rom_banks {
                      // This might happen if ROM size code was wrong or banking logic is off
                      // Return 0xFF is safer than panicking during emulation
                      return 0xFF;
                 }

                let rom_offset = (self.current_rom_bank * ROM_BANK_N_SIZE) + (addr - ROM_BANK_N_START) as usize;
                 if rom_offset < self.full_rom_data.len() {
                    self.full_rom_data[rom_offset]
                 } else {
                     // Access beyond the actual ROM data size
                    //  eprintln!("WARN: Read attempt beyond ROM size at bank {}, offset {}", self.current_rom_bank, rom_offset);
                     0xFF // Or some other default value?
                 }
            }
            // Video RAM (VRAM)
            VRAM_START..=VRAM_END => {
                // TODO: Check PPU mode restrictions
                self.vram[(addr - VRAM_START) as usize]
            }
            // External RAM / RTC Registers
            EXT_RAM_START..=EXT_RAM_END => {
                if !self.ram_enabled {
                    return 0xFF; // RAM disabled or not present
                }

                match self.mbc_type {
                     MbcType::Mbc3 if self.rtc_mapped_register >= 0x08 && self.rtc_mapped_register <= 0x0C => {
                         // Reading from latched RTC register
                         self.rtc_latched.read(self.rtc_mapped_register)
                     }
                     _ => { // Includes NoMbc, Mbc1, and Mbc3 RAM access
                        if !self.has_ram || self.external_ram.is_empty() {
                             return 0xFF; // No RAM present even if "enabled" conceptually
                        }
                        if self.current_ram_bank >= self.num_ram_banks {
                           // Attempt to access non-existent RAM bank
                           return 0xFF;
                        }
                        let ram_offset = (self.current_ram_bank * EXT_RAM_SIZE) + (addr - EXT_RAM_START) as usize;
                        if ram_offset < self.external_ram.len() {
                             self.external_ram[ram_offset]
                        } else {
                             // Should not happen if num_ram_banks is correct
                            //  eprintln!("WARN: Read attempt beyond RAM size at bank {}, offset {}", self.current_ram_bank, ram_offset);
                             0xFF
                        }
                     }
                }
            }
            // Work RAM Bank 0
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize]
            }
            // Work RAM Bank N (Fixed Bank 1 here, CGB would switch)
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize]
            }
            // Echo RAM
            ECHO_RAM_START..=ECHO_RAM_END => {
                self.read_byte(addr - 0x2000)
            }
            // OAM
            OAM_START..=OAM_END => {
                // TODO: Check PPU mode restrictions
                self.oam[(addr - OAM_START) as usize]
            }
            // Not Usable Area
            NOT_USABLE_START..=NOT_USABLE_END => 0xFF,
            // I/O Registers
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                // TODO: Handle read side effects (e.g., reading JOYP)
                self.io_registers[(addr - IO_REGISTERS_START) as usize]
            }
            // High RAM (HRAM)
            HRAM_START..=HRAM_END => {
                self.hram[(addr - HRAM_START) as usize]
            }
            // Interrupt Enable Register (IE)
            INTERRUPT_ENABLE_REGISTER => self.interrupt_enable,
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            // --- MBC Control Registers ---
            0x0000..=0x1FFF => { // RAM Enable (MBC1, MBC3) / RTC Register Select Enable (MBC3)
                 match self.mbc_type {
                    MbcType::Mbc1 | MbcType::Mbc3 => {
                        // Check if RAM (or RTC for MBC3) is present before enabling
                        if self.has_ram || self.mbc_type == MbcType::Mbc3 { // MBC3 always has RTC capability
                           self.ram_enabled = (value & 0x0F) == 0x0A;
                        //    println!("RAM/RTC Enable: {}", self.ram_enabled);
                        }
                    }
                    _ => {} // No effect for NoMbc
                 }
            }
            0x2000..=0x3FFF => { // ROM Bank Number (Lower bits)
                match self.mbc_type {
                    MbcType::Mbc1 => {
                         let bank_low = value & 0x1F; // Lower 5 bits
                         self.mbc1_rom_bank_lower = if bank_low == 0 { 1 } else { bank_low }; // Bank 0 -> 1
                         self.update_mbc1_rom_bank();
                    }
                    MbcType::Mbc3 => {
                         let bank = value & 0x7F; // 7 bits
                         self.current_rom_bank = if bank == 0 { 1 } else { bank as usize }; // Bank 0 -> 1
                         self.current_rom_bank &= (self.num_rom_banks - 1); // Mask to available banks
                        //  println!("MBC3 ROM Bank Set: {}", self.current_rom_bank);
                    }
                    _ => {} // No effect for NoMbc
                }
            }
            0x4000..=0x5FFF => { // RAM Bank Number / ROM Bank Upper Bits (MBC1) / RTC Register Select (MBC3)
                 match self.mbc_type {
                     MbcType::Mbc1 => {
                         self.mbc1_bank_upper = value & 0x03; // 2 bits used
                         self.update_mbc1_rom_bank(); // ROM bank might change
                         self.update_mbc1_ram_bank(); // RAM bank might change
                     }
                     MbcType::Mbc3 => {
                         if value <= 0x07 {
                            // Select RAM Bank
                            self.current_ram_bank = value as usize;
                            self.rtc_mapped_register = 0; // Indicate RAM is selected
                            self.current_ram_bank &= (self.num_ram_banks -1); // Mask if power of 2
                            if self.num_ram_banks > 0 && self.current_ram_bank >= self.num_ram_banks {
                                self.current_ram_bank %= self.num_ram_banks;
                            }
                            // println!("MBC3 RAM Bank Set: {}", self.current_ram_bank);
                         } else if value >= 0x08 && value <= 0x0C {
                             // Select RTC Register
                             self.rtc_mapped_register = value;
                            //  println!("MBC3 RTC Register Select: {:02X}", value);
                         } else {
                             // Invalid value, often ignored
                         }
                     }
                     _ => {} // No effect for NoMbc
                 }
            }
             0x6000..=0x7FFF => { // Banking Mode Select (MBC1) / Latch Clock Data (MBC3)
                 match self.mbc_type {
                     MbcType::Mbc1 => {
                         self.banking_mode = value & 0x01;
                         // Update banks immediately based on new mode
                         self.update_mbc1_rom_bank();
                         self.update_mbc1_ram_bank();
                        //  println!("MBC1 Mode Set: {}", self.banking_mode);
                     }
                     MbcType::Mbc3 => {
                         // RTC Latch sequence: write 0x00, then 0x01
                         if self.rtc_latch_state == 0 && value == 0x00 {
                             self.rtc_latch_state = 1;
                         } else if self.rtc_latch_state == 1 && value == 0x01 {
                             self.rtc_latch_state = 2; // Ready to latch
                             // Update internal RTC state before latching
                             self.rtc.update();
                             // Latch the *current* RTC state
                             self.rtc_latched = self.rtc.clone();
                            //  println!("MBC3 RTC Latched: {:?}", self.rtc_latched);
                             // Reset state for next latch
                             self.rtc_latch_state = 0;
                         } else {
                             // Any other value resets the sequence
                             self.rtc_latch_state = 0;
                         }
                     }
                    _ => {} // No effect for NoMbc
                 }
             }

            // --- Normal Memory Areas ---
            // Video RAM (VRAM)
            VRAM_START..=VRAM_END => {
                // TODO: Check PPU mode restrictions
                self.vram[(addr - VRAM_START) as usize] = value;
            }
            // External RAM / RTC Registers
            EXT_RAM_START..=EXT_RAM_END => {
                if !self.ram_enabled {
                    return; // Write ignored if RAM/RTC disabled
                }

                match self.mbc_type {
                     MbcType::Mbc3 if self.rtc_mapped_register >= 0x08 && self.rtc_mapped_register <= 0x0C => {
                         // Writing to live RTC register (not latched version)
                         self.rtc.write(self.rtc_mapped_register, value);
                        //  println!("RTC Write: Reg {:02X} = {:02X}", self.rtc_mapped_register, value);
                     }
                     _ => { // Includes NoMbc, Mbc1, and Mbc3 RAM access
                        if !self.has_ram || self.external_ram.is_empty() {
                             return; // No RAM present
                        }
                         if self.current_ram_bank >= self.num_ram_banks {
                           // Attempt to access non-existent RAM bank
                           return;
                        }
                        let ram_offset = (self.current_ram_bank * EXT_RAM_SIZE) + (addr - EXT_RAM_START) as usize;
                        if ram_offset < self.external_ram.len() {
                             self.external_ram[ram_offset] = value;
                        } else {
                             // Should not happen if num_ram_banks is correct
                            //  eprintln!("WARN: Write attempt beyond RAM size at bank {}, offset {}", self.current_ram_bank, ram_offset);
                        }
                     }
                }
            }
            // Work RAM Bank 0
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize] = value;
            }
            // Work RAM Bank N (Fixed Bank 1 here, CGB would switch)
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize] = value;
            }
            // Echo RAM
            ECHO_RAM_START..=ECHO_RAM_END => {
                self.write_byte(addr - 0x2000, value);
            }
            // OAM
            OAM_START..=OAM_END => {
                // TODO: Check PPU mode restrictions
                self.oam[(addr - OAM_START) as usize] = value;
            }
            // Not Usable Area
            NOT_USABLE_START..=NOT_USABLE_END => { /* Write Ignored */ }
            // I/O Registers
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                // TODO: Implement write side effects (DMA, Timer control, LCD control, etc.)
                let offset = (addr - IO_REGISTERS_START) as usize;

                 // --- VERY IMPORTANT: Add specific register handlers here ---
                 match addr {
                      0xFF04 => { // DIV - Writing any value resets it to 0
                         self.io_registers[offset] = 0;
                         // TODO: Reset internal timer divider counter
                      }
                      0xFF46 => { // DMA Transfer
                         self.io_registers[offset] = value;
                         self.perform_dma_transfer(value);
                      }
                      // Add handlers for 0xFF05(TIMA), 0xFF06(TMA), 0xFF07(TAC),
                      // 0xFF40(LCDC), 0xFF41(STAT), Sound Regs, etc.
                      _ => {
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

    fn perform_dma_transfer(&mut self, source_high_byte: u8) {
        // Source address is 0xXX00 where XX is the value written
        let source_start_addr = (source_high_byte as u16) << 8;

        // DMA copies 160 bytes (OAM_SIZE)
        for i in 0..OAM_SIZE {
             // Read from source (ROM, WRAM usually)
             let byte_to_copy = self.read_byte(source_start_addr + i as u16);
             // Write to OAM (destination is FE00 + i)
             // Directly write to OAM array, bypassing normal write_byte OAM check for speed
             // and to avoid potential PPU restrictions during DMA? (Check documentation)
             self.oam[i] = byte_to_copy;
        }
         // TODO: DMA takes time (160 machine cycles). The CPU should be blocked during this.
         //       This needs integration with the main emulator loop/CPU cycle counting.
    }

     // Helper method for debugging (optional)
    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

     // Helper method for debugging (optional)
    pub fn write_word(&mut self, addr: u16, value: u16) {
        let low = (value & 0xFF) as u8;
        let high = (value >> 8) as u8;
        self.write_byte(addr, low);
        self.write_byte(addr.wrapping_add(1), high);
    }

    // Public method to allow emulator loop to tick RTC
    pub fn tick_rtc(&mut self) {
        if self.mbc_type == MbcType::Mbc3 {
            self.rtc.update();
        }
    }
}

// Implement Debug for easier printing/logging
impl fmt::Debug for MemoryBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryBus")
         .field("mbc_type", &self.mbc_type)
         .field("rom_banks", &self.num_rom_banks)
         .field("ram_banks", &self.num_ram_banks)
         .field("has_ram", &self.has_ram)
         .field("ram_enabled", &self.ram_enabled)
         .field("current_rom_bank", &self.current_rom_bank)
         .field("current_ram_bank", &self.current_ram_bank)
         .field("banking_mode (MBC1)", &self.banking_mode)
         .field("rtc_mapped (MBC3)", &self.rtc_mapped_register)
         .field("interrupt_enable", &format_args!("{:#04X}", self.interrupt_enable))
         // Avoid printing large arrays
         .field("vram (size)", &self.vram.len())
         .field("external_ram (size)", &self.external_ram.len())
         .field("wram_bank_0 (size)", &self.wram_bank_0.len())
         .field("oam (size)", &self.oam.len())
         .field("hram (size)", &self.hram.len())
         .finish_non_exhaustive() // Indicates other fields exist but aren't shown
    }
}