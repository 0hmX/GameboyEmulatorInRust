#![allow(dead_code)] // Allow unused constants, as they define the complete map

// --- Memory Map Constants ---
pub const ROM_BANK_0_START: u16 = 0x0000;
pub const ROM_BANK_0_END: u16 = 0x3FFF;
pub const ROM_BANK_0_SIZE: usize = (ROM_BANK_0_END - ROM_BANK_0_START + 1) as usize; // 16 KiB

pub const ROM_BANK_N_START: u16 = 0x4000;
pub const ROM_BANK_N_END: u16 = 0x7FFF;
pub const ROM_BANK_N_SIZE: usize = (ROM_BANK_N_END - ROM_BANK_N_START + 1) as usize; // 16 KiB

pub const VRAM_START: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;
pub const VRAM_SIZE: usize = (VRAM_END - VRAM_START + 1) as usize; // 8 KiB

pub const EXT_RAM_START: u16 = 0xA000;
pub const EXT_RAM_END: u16 = 0xBFFF;
pub const EXT_RAM_SIZE: usize = (EXT_RAM_END - EXT_RAM_START + 1) as usize; // 8 KiB (standard bank size)

pub const WRAM_BANK_0_START: u16 = 0xC000;
pub const WRAM_BANK_0_END: u16 = 0xCFFF;
pub const WRAM_BANK_0_SIZE: usize = (WRAM_BANK_0_END - WRAM_BANK_0_START + 1) as usize; // 4 KiB

pub const WRAM_BANK_N_START: u16 = 0xD000;
pub const WRAM_BANK_N_END: u16 = 0xDFFF;
pub const WRAM_BANK_N_SIZE: usize = (WRAM_BANK_N_END - WRAM_BANK_N_START + 1) as usize; // 4 KiB (Bank 1 for DMG/CGB compat)

pub const ECHO_RAM_START: u16 = 0xE000;
pub const ECHO_RAM_END: u16 = 0xFDFF;
// ECHO_RAM_SIZE is derived from WRAM_BANK_0_SIZE + WRAM_BANK_N_SIZE * (Num Banks - 1),
// effectively mirroring C000-DDFF. No separate size constant needed.

pub const OAM_START: u16 = 0xFE00;
pub const OAM_END: u16 = 0xFE9F;
pub const OAM_SIZE: usize = (OAM_END - OAM_START + 1) as usize; // 160 bytes

pub const NOT_USABLE_START: u16 = 0xFEA0;
pub const NOT_USABLE_END: u16 = 0xFEFF;
// NOT_USABLE_SIZE is (NOT_USABLE_END - NOT_USABLE_START + 1) = 96 bytes.

pub const IO_REGISTERS_START: u16 = 0xFF00;
pub const IO_REGISTERS_END: u16 = 0xFF7F;
pub const IO_REGISTERS_SIZE: usize = (IO_REGISTERS_END - IO_REGISTERS_START + 1) as usize; // 128 bytes

pub const HRAM_START: u16 = 0xFF80;
pub const HRAM_END: u16 = 0xFFFE;
pub const HRAM_SIZE: usize = (HRAM_END - HRAM_START + 1) as usize; // 127 bytes

pub const INTERRUPT_ENABLE_REGISTER: u16 = 0xFFFF; // Single byte at this address

// --- Specific I/O Register Addresses ---
// Range: 0xFF00 - 0xFF7F
pub const P1_JOYP_ADDR: u16 = 0xFF00; // Joypad (R/W)
pub const SB_ADDR: u16 = 0xFF01; // Serial transfer data (R/W)
pub const SC_ADDR: u16 = 0xFF02; // Serial transfer control (R/W)
// 0xFF03 - Unused
pub const DIV_ADDR: u16 = 0xFF04; // Divider Register (R/W - Write resets to 0)
pub const TIMA_ADDR: u16 = 0xFF05; // Timer counter (R/W)
pub const TMA_ADDR: u16 = 0xFF06; // Timer Modulo (R/W)
pub const TAC_ADDR: u16 = 0xFF07; // Timer Control (R/W)
// 0xFF08 to 0xFF0E - Unused
pub const IF_ADDR: u16 = 0xFF0F; // Interrupt Flag (R/W)

// --- Sound Registers ---
// Range: 0xFF10 - 0xFF26 (NR registers), 0xFF30 - 0xFF3F (Wave Pattern RAM)
pub const NR10_ADDR: u16 = 0xFF10; // Channel 1 Sweep register (R/W)
pub const NR11_ADDR: u16 = 0xFF11; // Channel 1 Sound length/Wave pattern duty (R/W)
pub const NR12_ADDR: u16 = 0xFF12; // Channel 1 Volume Envelope (R/W)
pub const NR13_ADDR: u16 = 0xFF13; // Channel 1 Frequency lo (W)
pub const NR14_ADDR: u16 = 0xFF14; // Channel 1 Frequency hi (R/W)
pub const NR21_ADDR: u16 = 0xFF16; // Channel 2 Sound Length/Wave pattern duty (R/W)
pub const NR22_ADDR: u16 = 0xFF17; // Channel 2 Volume Envelope (R/W)
pub const NR23_ADDR: u16 = 0xFF18; // Channel 2 Frequency lo data (W)
pub const NR24_ADDR: u16 = 0xFF19; // Channel 2 Frequency hi data (R/W)
pub const NR30_ADDR: u16 = 0xFF1A; // Channel 3 Sound on/off (R/W)
pub const NR31_ADDR: u16 = 0xFF1B; // Channel 3 Sound Length (R/W)
pub const NR32_ADDR: u16 = 0xFF1C; // Channel 3 Select output level (R/W)
pub const NR33_ADDR: u16 = 0xFF1D; // Channel 3 Frequency's lower data (W)
pub const NR34_ADDR: u16 = 0xFF1E; // Channel 3 Frequency's higher data (R/W)
pub const NR41_ADDR: u16 = 0xFF20; // Channel 4 Sound Length (R/W)
pub const NR42_ADDR: u16 = 0xFF21; // Channel 4 Volume Envelope (R/W)
pub const NR43_ADDR: u16 = 0xFF22; // Channel 4 Polynomial Counter (R/W)
pub const NR44_ADDR: u16 = 0xFF23; // Channel 4 Counter/consecutive; Initial (R/W)
pub const NR50_ADDR: u16 = 0xFF24; // Channel control / ON-OFF / Volume (R/W)
pub const NR51_ADDR: u16 = 0xFF25; // Selection of Sound output terminal (R/W)
pub const NR52_ADDR: u16 = 0xFF26; // Sound on/off (R/W)
// 0xFF27 to 0xFF2F - Unused
pub const WAVE_PATTERN_RAM_START: u16 = 0xFF30;
pub const WAVE_PATTERN_RAM_END: u16 = 0xFF3F;
// WAVE_PATTERN_RAM_SIZE is 16 bytes.

// --- LCD Registers ---
// Range: 0xFF40 - 0xFF4B
pub const LCDC_ADDR: u16 = 0xFF40; // LCD Control (R/W)
pub const STAT_ADDR: u16 = 0xFF41; // LCD Status (R/W)
pub const SCY_ADDR: u16 = 0xFF42; // Scroll Y (R/W)
pub const SCX_ADDR: u16 = 0xFF43; // Scroll X (R/W)
pub const LY_ADDR: u16 = 0xFF44; // LCD Y Coordinate (R)
pub const LYC_ADDR: u16 = 0xFF45; // LY Compare (R/W)
pub const DMA_ADDR: u16 = 0xFF46; // DMA Transfer and Start Address (W)
pub const BGP_ADDR: u16 = 0xFF47; // BG Palette Data (R/W) - Non CGB
pub const OBP0_ADDR: u16 = 0xFF48; // Object Palette 0 Data (R/W) - Non CGB
pub const OBP1_ADDR: u16 = 0xFF49; // Object Palette 1 Data (R/W) - Non CGB
pub const WY_ADDR: u16 = 0xFF4A; // Window Y Position (R/W)
pub const WX_ADDR: u16 = 0xFF4B; // Window X Position plus 7 (R/W)
// 0xFF4C - Unused (Often KEY1 on CGB for Speed Switch)
// 0xFF4D - KEY1 (CGB Speed Switch)
// 0xFF4E - Unused
// 0xFF4F - VBK (CGB VRAM Bank Select)
// 0xFF50 - Boot ROM Disable (Write-only)
// 0xFF51 - HDMA1 (CGB HDMA Source High)
// 0xFF52 - HDMA2 (CGB HDMA Source Low)
// 0xFF53 - HDMA3 (CGB HDMA Destination High)
// 0xFF54 - HDMA4 (CGB HDMA Destination Low)
// 0xFF55 - HDMA5 (CGB HDMA Length/Mode/Start)
// 0xFF68 - BCPS/BGPI (CGB Background Palette Index)
// 0xFF69 - BCPD/BGPD (CGB Background Palette Data)
// 0xFF6A - OCPS/OBPI (CGB Object Palette Index)
// 0xFF6B - OCPD/OBPD (CGB Object Palette Data)
// 0xFF70 - SVBK (CGB WRAM Bank Select)
// ... other CGB registers up to 0xFF7F

// --- Interrupt Bits (for IF Register 0xFF0F and IE Register 0xFFFF) ---
// Bit position corresponds to the interrupt priority (0 = highest)
pub const VBLANK_INTERRUPT_BIT: u8 = 0; // V-Blank Interrupt
pub const LCD_STAT_INTERRUPT_BIT: u8 = 1; // LCD STAT Interrupt
pub const TIMER_INTERRUPT_BIT: u8 = 2; // Timer Interrupt
pub const SERIAL_INTERRUPT_BIT: u8 = 3; // Serial Interrupt
pub const JOYPAD_INTERRUPT_BIT: u8 = 4; // Joypad Interrupt
