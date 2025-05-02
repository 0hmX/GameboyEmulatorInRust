use crate::joypad::Joypad;
use crate::mbc::MbcType;
use crate::memory_map::*;
use crate::rtc::RtcRegisters;
use sdl2::keyboard::Keycode; // Keep for key_down/key_up method signature
use std::fmt;

/// Represents the Game Boy's memory map with MBC1/MBC3 support and input handling.
#[derive(Clone)]
pub struct MemoryBus {
    // Core Memory Areas
    rom_bank_0: Box<[u8; ROM_BANK_0_SIZE]>,
    vram: Box<[u8; VRAM_SIZE]>,
    wram_bank_0: Box<[u8; WRAM_BANK_0_SIZE]>,
    wram_bank_n: Box<[u8; WRAM_BANK_N_SIZE]>, // Always Bank 1 for DMG/CGB in non-CGB mode
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

    // Input State (delegated to Joypad struct)
    pub joypad: Joypad, // Made public to allow external calls to key_down/up

    // Calculated sizes (from ROM header)
    num_rom_banks: usize,
    num_ram_banks: usize,
}

impl MemoryBus {
    pub fn new() -> Self {
        // Initialize IO registers with known default values after boot ROM (if skipping)
        // Reference: Pandocs - Power Up Sequence
        let mut io_regs = [0u8; IO_REGISTERS_SIZE];
        // Note: P1 default is handled by Joypad::new() now
        io_regs[(SB_ADDR - IO_REGISTERS_START) as usize] = 0x00; // SB - Usually 0x00
        io_regs[(SC_ADDR - IO_REGISTERS_START) as usize] = 0x7E; // SC (Serial Control)
        io_regs[(DIV_ADDR - IO_REGISTERS_START) as usize] = 0xAC; // DIV - Arbitrary non-zero start? Pandocs doesn't specify post-bootrom exact value well? Often 0xAB or 0xAC used in emus.
        io_regs[(TIMA_ADDR - IO_REGISTERS_START) as usize] = 0x00; // TIMA
        io_regs[(TMA_ADDR - IO_REGISTERS_START) as usize] = 0x00; // TMA
        io_regs[(TAC_ADDR - IO_REGISTERS_START) as usize] = 0x00; // TAC // PanDocs says F0? Check boot ROM behavior. Let's stick to 00 for now.
        io_regs[(IF_ADDR - IO_REGISTERS_START) as usize] = 0xE1; // IF - Pandocs says 0xE1 after boot (VBL,LCD,TIM)
        io_regs[(NR10_ADDR - IO_REGISTERS_START) as usize] = 0x80; // NR10
        // TODO: Initialize NR11-NR52 with their defaults ...
        io_regs[(0xFF11 - IO_REGISTERS_START) as usize] = 0xBF; // NR11
        io_regs[(0xFF12 - IO_REGISTERS_START) as usize] = 0xF3; // NR12
        io_regs[(0xFF14 - IO_REGISTERS_START) as usize] = 0xBF; // NR14
        io_regs[(0xFF16 - IO_REGISTERS_START) as usize] = 0x3F; // NR21
        io_regs[(0xFF17 - IO_REGISTERS_START) as usize] = 0x00; // NR22
        io_regs[(0xFF19 - IO_REGISTERS_START) as usize] = 0xBF; // NR24
        io_regs[(0xFF1A - IO_REGISTERS_START) as usize] = 0x7F; // NR30
        io_regs[(0xFF1B - IO_REGISTERS_START) as usize] = 0xFF; // NR31
        io_regs[(0xFF1C - IO_REGISTERS_START) as usize] = 0x9F; // NR32
        io_regs[(0xFF1E - IO_REGISTERS_START) as usize] = 0xBF; // NR33
        io_regs[(0xFF20 - IO_REGISTERS_START) as usize] = 0xFF; // NR41
        io_regs[(0xFF21 - IO_REGISTERS_START) as usize] = 0x00; // NR42
        io_regs[(0xFF22 - IO_REGISTERS_START) as usize] = 0x00; // NR43
        io_regs[(0xFF23 - IO_REGISTERS_START) as usize] = 0xBF; // NR44
        io_regs[(0xFF24 - IO_REGISTERS_START) as usize] = 0x77; // NR50
        io_regs[(0xFF25 - IO_REGISTERS_START) as usize] = 0xF3; // NR51
        io_regs[(0xFF26 - IO_REGISTERS_START) as usize] = 0xF1; // NR52 - For GB
        // --- End Sound Regs ---
        io_regs[(LCDC_ADDR - IO_REGISTERS_START) as usize] = 0x91; // LCDC
        io_regs[(STAT_ADDR - IO_REGISTERS_START) as usize] = 0x85; // STAT - Mode 1 + LYC Flag set initially? Check boot ROM.
        io_regs[(SCY_ADDR - IO_REGISTERS_START) as usize] = 0x00; // SCY
        io_regs[(SCX_ADDR - IO_REGISTERS_START) as usize] = 0x00; // SCX
        io_regs[(LY_ADDR - IO_REGISTERS_START) as usize] = 0x00; // LY - PPU drives this, init 0. PanDocs shows 0x90 post bootrom, maybe related to frame timing? Let's init 0.
        io_regs[(LYC_ADDR - IO_REGISTERS_START) as usize] = 0x00; // LYC
        io_regs[(DMA_ADDR - IO_REGISTERS_START) as usize] = 0xFF; // DMA - Or 0x00? Let's use FF as unlikely value.
        io_regs[(BGP_ADDR - IO_REGISTERS_START) as usize] = 0xFC; // BGP Palette
        io_regs[(OBP0_ADDR - IO_REGISTERS_START) as usize] = 0xFF; // OBP0 Palette
        io_regs[(OBP1_ADDR - IO_REGISTERS_START) as usize] = 0xFF; // OBP1 Palette
        io_regs[(WY_ADDR - IO_REGISTERS_START) as usize] = 0x00; // WY
        io_regs[(WX_ADDR - IO_REGISTERS_START) as usize] = 0x00; // WX

        let mut bus = MemoryBus {
            rom_bank_0: Box::new([0; ROM_BANK_0_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            wram_bank_0: Box::new([0; WRAM_BANK_0_SIZE]),
            wram_bank_n: Box::new([0; WRAM_BANK_N_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            io_registers: Box::new(io_regs), // Use initialized IO regs
            hram: Box::new([0; HRAM_SIZE]),
            interrupt_enable: 0x00, // IE register starts at 0x00

            full_rom_data: Vec::new(),
            external_ram: Vec::new(),
            mbc_type: MbcType::NoMbc, // Default, overwritten by load_rom
            has_ram: false,
            has_battery: false,

            current_rom_bank: 1, // Default for banks 1-N
            current_ram_bank: 0,
            ram_enabled: false,
            banking_mode: 0,

            mbc1_rom_bank_lower: 1,
            mbc1_bank_upper: 0,

            rtc: RtcRegisters::new(), // Use constructor
            rtc_latched: RtcRegisters::default(), // Will be cloned on latch
            rtc_latch_state: 0,
            rtc_mapped_register: 0,

            joypad: Joypad::new(), // Initialize Joypad module

            num_rom_banks: 2, // Default (e.g., for 32KB ROM)
            num_ram_banks: 0,
        };

        // Write initial joypad state to P1 register
        let joyp_val = bus.joypad.read_p1();
        bus.io_registers[(P1_JOYP_ADDR - IO_REGISTERS_START) as usize] = joyp_val;

        bus
    }

    /// Loads ROM data and configures MBC based on the header.
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        if rom_data.len() < 0x150 {
            panic!("ROM data is too small to contain a valid header");
        }

        // Basic header info
        let cartridge_type_code = rom_data[0x0147];
        let rom_size_code = rom_data[0x0148];
        let ram_size_code = rom_data[0x0149];

        // Determine MBC Type, RAM, Battery using MbcType helper
        (self.mbc_type, self.has_ram, self.has_battery) =
            MbcType::from_header(cartridge_type_code);

        // Determine ROM size and number of banks
        self.num_rom_banks = match rom_size_code {
            0x00..=0x08 => 2 << rom_size_code, // 2, 4, 8, ..., 512
            0x52 => 72,
            0x53 => 80,
            0x54 => 96,
            _ => panic!("Unsupported ROM size code: {:02X}", rom_size_code),
        };
        let expected_rom_size = self.num_rom_banks * ROM_BANK_N_SIZE;
        if rom_data.len() < expected_rom_size {
            println!(
                "Warning: ROM file size ({}) is smaller than expected ({}) based on header.",
                rom_data.len(),
                expected_rom_size
            );
        }
        if rom_data.len() > expected_rom_size {
             println!(
                "Info: ROM file size ({}) is larger than expected ({}) based on header. Extra data might be ignored.",
                rom_data.len(),
                expected_rom_size
            );
        }

        // Determine RAM size and number of banks
        let ram_size = match ram_size_code {
            0x00 => 0,
            0x01 => 2 * 1024,   // 2 KiB (rarely used?)
            0x02 => 8 * 1024,   // 8 KiB (1 bank)
            0x03 => 32 * 1024,  // 32 KiB (4 banks)
            0x04 => 128 * 1024, // 128 KiB (16 banks)
            0x05 => 64 * 1024,  // 64 KiB (8 banks)
            _ => panic!("Unsupported RAM size code: {:02X}", ram_size_code),
        };

        // Consistency checks
        if ram_size > 0 && !self.has_ram {
            println!(
                "Warning: Cartridge header RAM size {:02X} indicates RAM, but type {:02X} usually lacks RAM.",
                ram_size_code, cartridge_type_code
            );
            // Decide how to handle: trust type or trust size? Let's trust type for now.
            // self.has_ram = true; // Option: trust size code
        }
        if ram_size == 0 && self.has_ram && self.mbc_type != MbcType::Mbc3 { // MBC3 might have RTC only
            println!(
                "Warning: Cartridge header RAM size 00, but type {:02X} usually expects RAM.",
                cartridge_type_code
            );
             // self.has_ram = false; // Option: trust size code
        }

        // Store ROM data
        self.full_rom_data = rom_data.to_vec();
        if self.full_rom_data.len() >= ROM_BANK_0_SIZE {
            self.rom_bank_0
                .copy_from_slice(&self.full_rom_data[0..ROM_BANK_0_SIZE]);
        } else {
            panic!("ROM is smaller than 16KB, cannot load into Bank 0.");
        }

        // Initialize External RAM
        if self.has_ram && ram_size > 0 {
            self.external_ram = vec![0u8; ram_size]; // Initialize with zeros
            // Calculate RAM banks based on standard 8KB bank size
            self.num_ram_banks = ram_size.max(EXT_RAM_SIZE) / EXT_RAM_SIZE;
        } else {
            // Ensure consistency if RAM isn't present or size is 0
            self.has_ram = false;
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
        self.rtc = RtcRegisters::new(); // Re-initialize RTC on load
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

    // --- MBC Helper Logic --- (Kept internal to MemoryBus for now)

    /// Updates the effective ROM bank for MBC1 based on current register values.
    fn update_mbc1_rom_bank(&mut self) {
        let mut bank = self.mbc1_rom_bank_lower as usize;
        if self.banking_mode == 0 {
            bank |= (self.mbc1_bank_upper as usize) << 5;
        }
        if bank == 0 || bank == 0x20 || bank == 0x40 || bank == 0x60 {
            bank += 1;
        }
        self.current_rom_bank = bank & (self.num_rom_banks.max(1) - 1); // .max(1) prevents subtract overflow if num_banks=0 somehow
    }

    /// Updates the effective RAM bank for MBC1 based on current register values.
    fn update_mbc1_ram_bank(&mut self) {
        if self.banking_mode == 1 {
            self.current_ram_bank = self.mbc1_bank_upper as usize;
        } else {
            self.current_ram_bank = 0;
        }
        if self.num_ram_banks > 0 {
            self.current_ram_bank &= (self.num_ram_banks - 1);
        } else {
            self.current_ram_bank = 0;
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

    pub fn read_byte(&self, addr: u16) -> u8 { // Make mutable for RTC latch read side-effect
        match addr {
            // ROM Bank 0 (Fixed)
            ROM_BANK_0_START..=ROM_BANK_0_END => self.rom_bank_0[addr as usize],
            // ROM Bank N (Switchable)
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                let effective_rom_bank = self.current_rom_bank % self.num_rom_banks.max(1);
                let rom_offset = (effective_rom_bank * ROM_BANK_N_SIZE)
                    + (addr - ROM_BANK_N_START) as usize;
                if rom_offset < self.full_rom_data.len() {
                    self.full_rom_data[rom_offset]
                } else {
                    0xFF // Access beyond actual ROM data size
                }
            }
            // Video RAM (VRAM)
            VRAM_START..=VRAM_END => {
                // TODO: Proper PPU mode checking
                self.vram[(addr - VRAM_START) as usize]
            }
            // External RAM / RTC Registers
            EXT_RAM_START..=EXT_RAM_END => {
                if !self.ram_enabled {
                    return 0xFF;
                }
                match self.mbc_type {
                    MbcType::Mbc3
                        if self.rtc_mapped_register >= 0x08 && self.rtc_mapped_register <= 0x0C =>
                    {
                        // Reading latched RTC register
                        self.rtc_latched.read(self.rtc_mapped_register)
                    }
                    _ => { // Includes NoMbc RAM, Mbc1 RAM, and Mbc3 RAM access
                        if !self.has_ram || self.external_ram.is_empty() || self.num_ram_banks == 0 {
                            return 0xFF;
                        }
                        let effective_ram_bank = self.current_ram_bank % self.num_ram_banks;
                        let ram_offset = (effective_ram_bank * EXT_RAM_SIZE)
                            + (addr - EXT_RAM_START) as usize;
                        if ram_offset < self.external_ram.len() {
                            self.external_ram[ram_offset]
                        } else {
                            0xFF // Should not happen if bounds checks are correct
                        }
                    }
                }
            }
            // Work RAM Bank 0
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize]
            }
            // Work RAM Bank N (Fixed Bank 1 on DMG)
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize]
            }
            // Echo RAM
            ECHO_RAM_START..=ECHO_RAM_END => self.read_byte(addr - 0x2000),
            // OAM
            OAM_START..=OAM_END => {
                 // TODO: Proper PPU mode checking
                self.oam[(addr - OAM_START) as usize]
            }
            // Not Usable Area
            NOT_USABLE_START..=NOT_USABLE_END => 0xFF, // Often reads 0xFF
            // I/O Registers
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                let offset = (addr - IO_REGISTERS_START) as usize;
                match addr {
                    P1_JOYP_ADDR => self.joypad.read_p1(), // Delegate to Joypad module
                    // Add reads for other registers that have side effects or specific behavior
                    STAT_ADDR => self.io_registers[offset] | 0x80, // Bit 7 always high
                    DIV_ADDR | TIMA_ADDR | TMA_ADDR | TAC_ADDR | IF_ADDR | LCDC_ADDR |
                    SCY_ADDR | SCX_ADDR | LY_ADDR | LYC_ADDR | DMA_ADDR | BGP_ADDR |
                    OBP0_ADDR | OBP1_ADDR | WY_ADDR | WX_ADDR |
                    0xFF10..=0xFF26 | 0xFF30..=0xFF3F // Sound Regs placeholder
                    => {
                        // TODO: Some registers might have read side-effects or depend on component state
                        self.io_registers[offset]
                    }
                    _ => self.io_registers[offset], // Default read
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
            0x0000..=0x1FFF => { // RAM/RTC Enable
                match self.mbc_type {
                    MbcType::Mbc1 | MbcType::Mbc3 => {
                        // Only enable if cart has RAM or it's MBC3 (for RTC)
                        if self.has_ram || (self.has_battery && self.mbc_type == MbcType::Mbc3) {
                            self.ram_enabled = (value & 0x0F) == 0x0A;
                        }
                    }
                    _ => {}
                }
            }
            0x2000..=0x3FFF => { // ROM Bank Number (Lower)
                match self.mbc_type {
                    MbcType::Mbc1 => {
                        let bank_low = value & 0x1F;
                        self.mbc1_rom_bank_lower = if bank_low == 0 { 1 } else { bank_low };
                        self.update_mbc1_rom_bank();
                    }
                    MbcType::Mbc3 => {
                        let bank = value & 0x7F;
                        self.current_rom_bank = if bank == 0 { 1 } else { bank as usize };
                        self.current_rom_bank &= (self.num_rom_banks.max(1) - 1);
                    }
                    _ => {}
                }
            }
            0x4000..=0x5FFF => { // RAM Bank / ROM Bank Upper (MBC1) / RTC Select (MBC3)
                match self.mbc_type {
                    MbcType::Mbc1 => {
                        self.mbc1_bank_upper = value & 0x03;
                        self.update_mbc1_rom_bank();
                        self.update_mbc1_ram_bank();
                    }
                    MbcType::Mbc3 => {
                        if value <= 0x07 { // Select RAM Bank 0-x
                            self.current_ram_bank = value as usize;
                            self.rtc_mapped_register = 0; // Indicate RAM selected
                             if self.num_ram_banks > 0 {
                                self.current_ram_bank &= (self.num_ram_banks - 1);
                            } else {
                                self.current_ram_bank = 0;
                            }
                        } else if (0x08..=0x0C).contains(&value) { // Select RTC Register
                            self.rtc_mapped_register = value;
                        } else { /* Invalid */ }
                    }
                    _ => {}
                }
            }
            0x6000..=0x7FFF => { // Banking Mode (MBC1) / Latch RTC (MBC3)
                match self.mbc_type {
                    MbcType::Mbc1 => {
                        self.banking_mode = value & 0x01;
                        self.update_mbc1_rom_bank();
                        self.update_mbc1_ram_bank();
                    }
                    MbcType::Mbc3 => {
                        // Latch sequence: write 0x00 then 0x01
                        if self.rtc_latch_state == 0 && value == 0x00 {
                            self.rtc_latch_state = 1;
                        } else if self.rtc_latch_state == 1 && value == 0x01 {
                            self.rtc.update(); // Ensure RTC state is current before latching
                            self.rtc_latched = self.rtc.clone();
                            self.rtc_latch_state = 0; // Reset
                        } else {
                            self.rtc_latch_state = 0; // Invalid sequence, reset
                        }
                    }
                    _ => {}
                }
            }

            // --- Normal Memory Areas ---
            VRAM_START..=VRAM_END => {
                // TODO: PPU Mode check
                self.vram[(addr - VRAM_START) as usize] = value;
            }
            EXT_RAM_START..=EXT_RAM_END => {
                if !self.ram_enabled { return; }
                match self.mbc_type {
                    MbcType::Mbc3 if self.rtc_mapped_register >= 0x08 => {
                        // Writing to live RTC register
                        self.rtc.write(self.rtc_mapped_register, value);
                    }
                    _ => { // RAM access
                        if !self.has_ram || self.external_ram.is_empty() || self.num_ram_banks == 0 { return; }
                         let effective_ram_bank = self.current_ram_bank % self.num_ram_banks;
                        let ram_offset = (effective_ram_bank * EXT_RAM_SIZE)
                            + (addr - EXT_RAM_START) as usize;
                        if ram_offset < self.external_ram.len() {
                            self.external_ram[ram_offset] = value;
                        }
                    }
                }
            }
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize] = value;
            }
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize] = value;
            }
            ECHO_RAM_START..=ECHO_RAM_END => self.write_byte(addr - 0x2000, value),
            OAM_START..=OAM_END => {
                // TODO: PPU Mode check
                self.oam[(addr - OAM_START) as usize] = value;
            }
            NOT_USABLE_START..=NOT_USABLE_END => { /* Write Ignored */ }
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                let offset = (addr - IO_REGISTERS_START) as usize;
                match addr {
                    P1_JOYP_ADDR => self.joypad.write_p1(value), // Delegate
                    DIV_ADDR => {
                        // TODO: Reset internal timer divider counter
                        self.io_registers[offset] = 0; // Write resets register
                    }
                    TIMA_ADDR | TMA_ADDR | TAC_ADDR => {
                        // TODO: Notify Timer component
                        self.io_registers[offset] = value;
                    }
                    IF_ADDR => {
                         // Bits 0-4 are R/W, bits 5-7 are unused (read as 1)
                         // Writing 1 to a flag bit *requests* it clear (unlike hardware where it clears directly)
                         // The CPU interrupt handling logic should clear flags after servicing.
                         // For direct writes (e.g. game code), allow writing 0 or 1 to flags.
                         // Let's preserve upper bits on write.
                        self.io_registers[offset] = (value & 0x1F) | (self.io_registers[offset] & 0xE0);
                    }
                    LCDC_ADDR | STAT_ADDR | SCY_ADDR | SCX_ADDR | LYC_ADDR | BGP_ADDR |
                    OBP0_ADDR | OBP1_ADDR | WY_ADDR | WX_ADDR => {
                        // TODO: Notify PPU component
                        self.io_registers[offset] = value;
                        if addr == STAT_ADDR {
                             // Preserve read-only bits (lower 3, mode flags)
                             // Only bits 3-6 (interrupt enables) are writable
                             // Bit 7 is always 1 (read).
                             // Let PPU manage bits 0,1,2. We only write bits 3-6 here.
                             self.io_registers[offset] = (value & 0b0111_1000) | (self.io_registers[offset] & 0b1000_0111);
                        }
                    }
                    DMA_ADDR => {
                        self.io_registers[offset] = value;
                        self.perform_dma_transfer(value);
                    }
                     0xFF10..=0xFF26 | 0xFF30..=0xFF3F => { // Sound Regs
                         // TODO: Notify APU component
                         self.io_registers[offset] = value;
                         // Some sound regs have write side effects (e.g., Trigger bit)
                     }
                    _ => { // Default write for unhandled/simple IO regs
                        self.io_registers[offset] = value;
                    }
                }
            }
            HRAM_START..=HRAM_END => {
                self.hram[(addr - HRAM_START) as usize] = value;
            }
            INTERRUPT_ENABLE_REGISTER => {
                self.interrupt_enable = value & 0x1F; // Only lower 5 bits used
            }
        }
    }

    // --- Helper methods ---

    /// Performs an OAM DMA transfer.
    fn perform_dma_transfer(&mut self, source_high_byte: u8) {
        // TODO: This should block CPU access to most memory for ~160 machine cycles.
        // This simplified version performs the copy instantly.
        let source_start_addr = (source_high_byte as u16) << 8;
        if source_start_addr >= 0xFE00 && source_start_addr <= 0xFFFF {
             // DMA from OAM/IO/HRAM/IE is often restricted or has weird behavior.
             // Let's prevent it from these areas for now. Common sources are 0x0000-0xDFFF.
             // Pandocs: "DMA source cannot be HRAM (FF80-FFFE)" - Let's block FE00+ entirely.
             println!("Warning: DMA Transfer requested from restricted area {:04X}", source_start_addr);
             return;
        }

        // Use read_byte to respect potential banking / read side effects (though DMA might bypass some?)
        for i in 0..OAM_SIZE {
            let byte_to_copy = self.read_byte(source_start_addr + i as u16);
            // Direct write to OAM array, bypassing potential PPU lockouts? Check details.
            self.oam[i] = byte_to_copy;
        }
        // Add CPU stall logic here or return required stall cycles.
    }

    /// Reads a 16-bit word (Little Endian).
    pub fn read_word(&mut self, addr: u16) -> u16 { // Mutable because read_byte is mutable
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    /// Writes a 16-bit word (Little Endian).
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

    // --- Public Input Handling Methods (Delegate to Joypad) ---

    /// Called by the frontend when a key mapped to a Game Boy button is pressed down.
    pub fn key_down(&mut self, key: Keycode) {
         if self.joypad.key_down(key) {
            self.request_interrupt(JOYPAD_INTERRUPT_BIT);
         }
         // Update P1 register reflecting the new button state immediately for polling reads
         let p1_val = self.joypad.read_p1();
         self.io_registers[(P1_JOYP_ADDR - IO_REGISTERS_START) as usize] = p1_val;
    }

    /// Called by the frontend when a key mapped to a Game Boy button is released.
    pub fn key_up(&mut self, key: Keycode) {
        self.joypad.key_up(key);
         // Update P1 register reflecting the new button state immediately for polling reads
         let p1_val = self.joypad.read_p1();
         self.io_registers[(P1_JOYP_ADDR - IO_REGISTERS_START) as usize] = p1_val;
    }

     // --- Debug / Accessor methods ---
     pub fn get_io_reg(&self, addr: u16) -> u8 {
        if (IO_REGISTERS_START..=IO_REGISTERS_END).contains(&addr) {
            let offset = (addr - IO_REGISTERS_START) as usize;
            self.io_registers[offset]
        } else if addr == INTERRUPT_ENABLE_REGISTER {
            self.interrupt_enable
        }
         else {
            0xFF // Or panic?
        }
     }

    // Need mutable access to internal IO regs for components like PPU/Timer?
    // Be careful with direct mutation vs using write_byte logic.
    pub fn set_io_reg_direct(&mut self, addr: u16, value: u8) {
         if (IO_REGISTERS_START..=IO_REGISTERS_END).contains(&addr) {
            let offset = (addr - IO_REGISTERS_START) as usize;
            // Direct write, bypasses write_byte logic (use with caution!)
            self.io_registers[offset] = value;
        } else if addr == INTERRUPT_ENABLE_REGISTER {
             self.interrupt_enable = value & 0x1F;
        }
         // Add more cases (like IF register?) if needed by other components
    }

    // Getter for VRAM needed by PPU
    pub fn get_vram(&self) -> &[u8; VRAM_SIZE] {
        &self.vram
    }
     // Getter for OAM needed by PPU
    pub fn get_oam(&self) -> &[u8; OAM_SIZE] {
        &self.oam
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
            .field("IE", &format_args!("{:#04X}", self.interrupt_enable))
            .field("IF", &format_args!("{:#04X}", self.get_io_reg(IF_ADDR)))
            .field("joypad", &self.joypad) // Show Joypad module state
            // Avoid printing large arrays
            .finish_non_exhaustive()
    }
}