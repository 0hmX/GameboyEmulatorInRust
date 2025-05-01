use std::fmt;

// Define constants for memory regions and sizes for clarity
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
const EXT_RAM_SIZE: usize = 0x2000; // 8 KiB

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

/// Represents the Game Boy's memory map.
///
/// ## Simplifications:
/// - No MBC (Memory Bank Controller) logic. Assumes a fixed 32KB ROM map
///   and potentially 8KB of external RAM. ROM Bank N is fixed.
/// - No CGB VRAM/WRAM bank switching implemented.
/// - I/O Registers are simple memory locations; writes have no side effects.
/// - PPU VRAM/OAM access restrictions are not implemented.
/// - "Not Usable" memory reads 0xFF and writes are ignored.
/// - Echo RAM correctly mirrors WRAM.
#[derive(Clone)] // Deriving Clone for easier state management if needed later
pub struct MemoryBus {
    // Using fixed-size arrays for memory areas with known, fixed sizes.
    rom_bank_0: Box<[u8; ROM_BANK_0_SIZE]>,   // 0000-3FFF (Cartridge ROM)
    rom_bank_n: Box<[u8; ROM_BANK_N_SIZE]>,   // 4000-7FFF (Cartridge ROM, usually switchable)
    vram: Box<[u8; VRAM_SIZE]>,               // 8000-9FFF (Video RAM)
    external_ram: Box<[u8; EXT_RAM_SIZE]>,    // A000-BFFF (Cartridge RAM, if present)
    wram_bank_0: Box<[u8; WRAM_BANK_0_SIZE]>, // C000-CFFF (Work RAM Bank 0)
    wram_bank_n: Box<[u8; WRAM_BANK_N_SIZE]>, // D000-DFFF (Work RAM Bank 1-7, CGB only)
    oam: Box<[u8; OAM_SIZE]>,                 // FE00-FE9F (Object Attribute Memory)
    io_registers: Box<[u8; IO_REGISTERS_SIZE]>, // FF00-FF7F (I/O Ports)
    hram: Box<[u8; HRAM_SIZE]>,               // FF80-FFFE (High RAM)
    interrupt_enable: u8,                     // FFFF (Interrupt Enable Register)

    // Potential future extension: Store the full ROM data
    // full_rom_data: Vec<u8>,
    // current_rom_bank: usize,
    // has_external_ram: bool, // etc.
}

impl MemoryBus {
    /// Creates a new `MemoryBus` instance with all RAM initialized to zero.
    /// ROM banks are initialized to zero until loaded.
    pub fn new() -> Self {
        MemoryBus {
            rom_bank_0: Box::new([0; ROM_BANK_0_SIZE]),
            rom_bank_n: Box::new([0; ROM_BANK_N_SIZE]), // Placeholder, load ROM to fill
            vram: Box::new([0; VRAM_SIZE]),
            external_ram: Box::new([0; EXT_RAM_SIZE]), // Assumes RAM present for simplicity
            wram_bank_0: Box::new([0; WRAM_BANK_0_SIZE]),
            wram_bank_n: Box::new([0; WRAM_BANK_N_SIZE]), // For CGB, DMG leaves this mostly unused
            oam: Box::new([0; OAM_SIZE]),
            // TODO: Initialize I/O registers to their specific boot values
            // For now, initializing to 0 is functionally okay for basic structure.
            io_registers: Box::new([0; IO_REGISTERS_SIZE]),
            hram: Box::new([0; HRAM_SIZE]),
            interrupt_enable: 0,
            // full_rom_data: Vec::new(),
            // current_rom_bank: 1, // Default bank 1
            // has_external_ram: false,
        }
    }

    /// Loads a ROM image into the memory bus.
    ///
    /// This basic version loads the first 32KB (Bank 0 and Bank 1).
    /// It panics if the ROM is smaller than 32KB.
    /// More advanced loading requires MBC handling.
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        if rom_data.len() < ROM_BANK_0_SIZE + ROM_BANK_N_SIZE {
            panic!(
                "ROM is too small. Minimum size is {} bytes.",
                ROM_BANK_0_SIZE + ROM_BANK_N_SIZE
            );
        }

        // Load ROM Bank 0
        self.rom_bank_0
            .copy_from_slice(&rom_data[0..ROM_BANK_0_SIZE]);

        // Load ROM Bank 1 (as the initial fixed Bank N)
        self.rom_bank_n
            .copy_from_slice(&rom_data[ROM_BANK_0_SIZE..(ROM_BANK_0_SIZE + ROM_BANK_N_SIZE)]);

        // In a real emulator, you would store all rom_data and implement
        // MBC logic to switch banks into rom_bank_n based on writes to
        // specific ROM address ranges.
        // self.full_rom_data = rom_data.to_vec();
        // self.current_rom_bank = 1; // Reset to bank 1 on load
        println!(
            "Loaded ROM: {} bytes (using first 32KB)",
            rom_data.len()
        );
    }

    /// Reads a byte from the specified memory address.
    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // ROM Bank 0 (Fixed)
            ROM_BANK_0_START..=ROM_BANK_0_END => {
                self.rom_bank_0[(addr - ROM_BANK_0_START) as usize]
            }
            // ROM Bank N (Switchable, but fixed in this simple version)
            ROM_BANK_N_START..=ROM_BANK_N_END => {
                // TODO: Implement MBC logic to select the correct bank
                self.rom_bank_n[(addr - ROM_BANK_N_START) as usize]
            }
            // Video RAM (VRAM)
            VRAM_START..=VRAM_END => {
                // TODO: Check PPU mode restrictions if implementing PPU
                self.vram[(addr - VRAM_START) as usize]
            }
            // External RAM (Cartridge RAM)
            EXT_RAM_START..=EXT_RAM_END => {
                // TODO: Check if RAM is enabled via MBC
                // if self.has_external_ram && self.is_ram_enabled {
                self.external_ram[(addr - EXT_RAM_START) as usize]
                // } else { 0xFF } // Typically reads FF if RAM disabled/absent
            }
            // Work RAM Bank 0
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize]
            }
            // Work RAM Bank N (Switchable on CGB)
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                // TODO: Implement CGB WRAM bank switching
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize]
            }
            // Echo RAM (Mirror of C000-DDFF)
            ECHO_RAM_START..=ECHO_RAM_END => {
                // Mirrors C000-DDFF (which includes WRAM Bank 0 and N)
                self.read_byte(addr - 0x2000)
            }
            // Object Attribute Memory (OAM)
            OAM_START..=OAM_END => {
                // TODO: Check PPU mode restrictions if implementing PPU
                self.oam[(addr - OAM_START) as usize]
            }
            // Not Usable Area
            NOT_USABLE_START..=NOT_USABLE_END => {
                // Behavior varies, often returns 0xFF or open bus value.
                // Return 0xFF as a common default.
                0xFF
            }
            // I/O Registers
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                // TODO: Implement read side effects and specific register behaviors
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

    /// Writes a byte to the specified memory address.
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            // ROM Area (Writes typically ignored or used for MBC control)
            ROM_BANK_0_START..=ROM_BANK_N_END => {
                // TODO: Implement MBC register writes here.
                // For now, writes to ROM are ignored.
                // eprintln!("Attempted write to ROM area: {:04X} = {:02X}", addr, value);
            }
            // Video RAM (VRAM)
            VRAM_START..=VRAM_END => {
                // TODO: Check PPU mode restrictions if implementing PPU
                self.vram[(addr - VRAM_START) as usize] = value;
            }
            // External RAM (Cartridge RAM)
            EXT_RAM_START..=EXT_RAM_END => {
                // TODO: Check if RAM is enabled via MBC
                // if self.has_external_ram && self.is_ram_enabled {
                    self.external_ram[(addr - EXT_RAM_START) as usize] = value;
                // }
            }
            // Work RAM Bank 0
            WRAM_BANK_0_START..=WRAM_BANK_0_END => {
                self.wram_bank_0[(addr - WRAM_BANK_0_START) as usize] = value;
            }
            // Work RAM Bank N (Switchable on CGB)
            WRAM_BANK_N_START..=WRAM_BANK_N_END => {
                // TODO: Implement CGB WRAM bank switching
                self.wram_bank_n[(addr - WRAM_BANK_N_START) as usize] = value;
            }
            // Echo RAM (Mirror of C000-DDFF)
            ECHO_RAM_START..=ECHO_RAM_END => {
                // Mirrors writes to C000-DDFF
                self.write_byte(addr - 0x2000, value);
            }
            // Object Attribute Memory (OAM)
            OAM_START..=OAM_END => {
                // TODO: Check PPU mode restrictions if implementing PPU
                self.oam[(addr - OAM_START) as usize] = value;
            }
            // Not Usable Area
            NOT_USABLE_START..=NOT_USABLE_END => {
                // Writes are typically ignored.
            }
            // I/O Registers
            IO_REGISTERS_START..=IO_REGISTERS_END => {
                // TODO: Implement write side effects and specific register behaviors!
                //       This is a major simplification. e.g., writing to DMA triggers transfer.
                //       Writing to STAT might have immediate effects, etc.
                 let offset = (addr - IO_REGISTERS_START) as usize;
                 self.io_registers[offset] = value;
                 // Add specific handlers here later, e.g.:
                 // if addr == 0xFF04 { // DIV Register
                 //     self.io_registers[offset] = 0; // Writing any value resets DIV
                 // } else {
                 //     self.io_registers[offset] = value;
                 // }
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
}

// Implement Debug for easier printing/logging
impl fmt::Debug for MemoryBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryBus")
         .field("rom_bank_0 (size)", &self.rom_bank_0.len())
         .field("rom_bank_n (size)", &self.rom_bank_n.len())
         .field("vram (size)", &self.vram.len())
         .field("external_ram (size)", &self.external_ram.len())
         .field("wram_bank_0 (size)", &self.wram_bank_0.len())
         .field("wram_bank_n (size)", &self.wram_bank_n.len())
         .field("oam (size)", &self.oam.len())
         .field("io_registers (size)", &self.io_registers.len())
         .field("hram (size)", &self.hram.len())
         .field("interrupt_enable", &self.interrupt_enable)
         .finish()
    }
}
