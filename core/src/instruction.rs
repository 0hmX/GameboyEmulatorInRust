use crate::cpu::{Cpu, CpuResult};
use crate::memory_bus::MemoryBus;
use lazy_static::lazy_static; // Add lazy_static = "1.4.0" to Cargo.toml

// Definition from your previous code block
#[derive(Clone)]
pub struct Instruction {
    pub mnemonic: &'static str,
    pub length: u8,
    pub cycles: u8, // Base T-cycles (minimum for conditional)
    pub execute: fn(&mut Cpu, &mut MemoryBus) -> CpuResult<u16>, // Returns *additional* T-cycles or Error
}

impl Instruction {
    pub const fn new(
        mnemonic: &'static str,
        length: u8,
        cycles: u8,
        execute: fn(&mut Cpu, &mut MemoryBus) -> CpuResult<u16>,
    ) -> Self {
        Instruction {
            mnemonic,
            length,
            cycles,
            execute,
        }
    }

    // Use the dedicated handler for invalid opcodes
    pub const fn invalid() -> Self {
        Instruction {
            mnemonic: "INVALID",
            length: 1,
            cycles: 4, // Base cycles consumed even on error path? Or handle in step? Let's assume 4.
            execute: Cpu::handle_invalid_opcode, // Points to the error handler
        }
    }
}

// Helper macro for creating instruction entries (optional, but reduces boilerplate)
macro_rules! instr {
    ($mne:expr, $len:expr, $cyc:expr, $exec:expr) => {
        Instruction::new($mne, $len, $cyc, $exec)
    };
}
macro_rules! invalid {
    () => {
        Instruction::invalid()
    };
}

lazy_static! {
    // Main instruction table (0x00 - 0xFF)
    pub static ref INSTRUCTIONS: [Instruction; 256] = [
        // --- 0x00 ---
        instr!("NOP", 1, 4, Cpu::op_nop),               // 00 NOP
        instr!("LD BC, d16", 3, 12, Cpu::op_ld_bc_d16), // 01 LD BC, d16
        instr!("LD (BC), A", 1, 8, Cpu::op_ld_bc_a),    // 02 LD (BC), A
        instr!("INC BC", 1, 8, Cpu::op_inc_bc),         // 03 INC BC
        instr!("INC B", 1, 4, Cpu::op_inc_b),           // 04 INC B
        instr!("DEC B", 1, 4, Cpu::op_dec_b),           // 05 DEC B
        instr!("LD B, d8", 2, 8, Cpu::op_ld_b_d8),      // 06 LD B, d8
        instr!("RLCA", 1, 4, Cpu::op_rlca),             // 07 RLCA
        instr!("LD (a16), SP", 3, 20, Cpu::op_ld_a16_sp), // 08 LD (a16), SP
        instr!("ADD HL, BC", 1, 8, Cpu::op_add_hl_bc),   // 09 ADD HL, BC
        instr!("LD A, (BC)", 1, 8, Cpu::op_ld_a_bc),    // 0A LD A, (BC)
        instr!("DEC BC", 1, 8, Cpu::op_dec_bc),         // 0B DEC BC
        instr!("INC C", 1, 4, Cpu::op_inc_c),           // 0C INC C
        instr!("DEC C", 1, 4, Cpu::op_dec_c),           // 0D DEC C
        instr!("LD C, d8", 2, 8, Cpu::op_ld_c_d8),      // 0E LD C, d8
        instr!("RRCA", 1, 4, Cpu::op_rrca),             // 0F RRCA
        // --- 0x10 ---
        instr!("STOP", 2, 4, Cpu::op_stop),             // 10 STOP 00
        instr!("LD DE, d16", 3, 12, Cpu::op_ld_de_d16), // 11 LD DE, d16
        instr!("LD (DE), A", 1, 8, Cpu::op_ld_de_a),    // 12 LD (DE), A
        instr!("INC DE", 1, 8, Cpu::op_inc_de),         // 13 INC DE
        instr!("INC D", 1, 4, Cpu::op_inc_d),           // 14 INC D
        instr!("DEC D", 1, 4, Cpu::op_dec_d),           // 15 DEC D
        instr!("LD D, d8", 2, 8, Cpu::op_ld_d_d8),      // 16 LD D, d8
        instr!("RLA", 1, 4, Cpu::op_rla),               // 17 RLA
        instr!("JR r8", 2, 12, Cpu::op_jr_r8),          // 18 JR r8 (Always takes 12 cycles)
        instr!("ADD HL, DE", 1, 8, Cpu::op_add_hl_de),   // 19 ADD HL, DE
        instr!("LD A, (DE)", 1, 8, Cpu::op_ld_a_de),    // 1A LD A, (DE)
        instr!("DEC DE", 1, 8, Cpu::op_dec_de),         // 1B DEC DE
        instr!("INC E", 1, 4, Cpu::op_inc_e),           // 1C INC E
        instr!("DEC E", 1, 4, Cpu::op_dec_e),           // 1D DEC E
        instr!("LD E, d8", 2, 8, Cpu::op_ld_e_d8),      // 1E LD E, d8
        instr!("RRA", 1, 4, Cpu::op_rra),               // 1F RRA
        // --- 0x20 --- JR NZ,r8 (8 cycles if no jump, 12 if jump)
        instr!("JR NZ, r8", 2, 8, Cpu::op_jr_nz_r8),    // 20 JR NZ, r8
        instr!("LD HL, d16", 3, 12, Cpu::op_ld_hl_d16), // 21 LD HL, d16
        instr!("LD (HL+), A", 1, 8, Cpu::op_ld_hli_a),  // 22 LD (HL+), A
        instr!("INC HL", 1, 8, Cpu::op_inc_hl),         // 23 INC HL
        instr!("INC H", 1, 4, Cpu::op_inc_h),           // 24 INC H
        instr!("DEC H", 1, 4, Cpu::op_dec_h),           // 25 DEC H
        instr!("LD H, d8", 2, 8, Cpu::op_ld_h_d8),      // 26 LD H, d8
        instr!("DAA", 1, 4, Cpu::op_daa),               // 27 DAA
        // --- 0x28 --- JR Z,r8 (8 cycles if no jump, 12 if jump)
        instr!("JR Z, r8", 2, 8, Cpu::op_jr_z_r8),      // 28 JR Z, r8
        instr!("ADD HL, HL", 1, 8, Cpu::op_add_hl_hl),   // 29 ADD HL, HL
        instr!("LD A, (HL+)", 1, 8, Cpu::op_ld_a_hli),  // 2A LD A, (HL+)
        instr!("DEC HL", 1, 8, Cpu::op_dec_hl),         // 2B DEC HL
        instr!("INC L", 1, 4, Cpu::op_inc_l),           // 2C INC L
        instr!("DEC L", 1, 4, Cpu::op_dec_l),           // 2D DEC L
        instr!("LD L, d8", 2, 8, Cpu::op_ld_l_d8),      // 2E LD L, d8
        instr!("CPL", 1, 4, Cpu::op_cpl),               // 2F CPL
        // --- 0x30 --- JR NC,r8 (8 cycles if no jump, 12 if jump)
        instr!("JR NC, r8", 2, 8, Cpu::op_jr_nc_r8),    // 30 JR NC, r8
        instr!("LD SP, d16", 3, 12, Cpu::op_ld_sp_d16), // 31 LD SP, d16
        instr!("LD (HL-), A", 1, 8, Cpu::op_ld_hld_a),  // 32 LD (HL-), A
        instr!("INC SP", 1, 8, Cpu::op_inc_sp),         // 33 INC SP
        instr!("INC (HL)", 1, 12, Cpu::op_inc_hlp),     // 34 INC (HL)
        instr!("DEC (HL)", 1, 12, Cpu::op_dec_hlp),     // 35 DEC (HL)
        instr!("LD (HL), d8", 2, 12, Cpu::op_ld_hlp_d8), // 36 LD (HL), d8
        instr!("SCF", 1, 4, Cpu::op_scf),               // 37 SCF
        // --- 0x38 --- JR C,r8 (8 cycles if no jump, 12 if jump)
        instr!("JR C, r8", 2, 8, Cpu::op_jr_c_r8),      // 38 JR C, r8
        instr!("ADD HL, SP", 1, 8, Cpu::op_add_hl_sp),   // 39 ADD HL, SP
        instr!("LD A, (HL-)", 1, 8, Cpu::op_ld_a_hld),  // 3A LD A, (HL-)
        instr!("DEC SP", 1, 8, Cpu::op_dec_sp),         // 3B DEC SP
        instr!("INC A", 1, 4, Cpu::op_inc_a),           // 3C INC A
        instr!("DEC A", 1, 4, Cpu::op_dec_a),           // 3D DEC A
        instr!("LD A, d8", 2, 8, Cpu::op_ld_a_d8),      // 3E LD A, d8
        instr!("CCF", 1, 4, Cpu::op_ccf),               // 3F CCF

        // --- 0x40..0x7F: LD r, r' ---
        // 0x40 - 0x47 : LD B, r
        instr!("LD B, B", 1, 4, Cpu::op_ld_b_b),
        instr!("LD B, C", 1, 4, Cpu::op_ld_b_c),
        instr!("LD B, D", 1, 4, Cpu::op_ld_b_d),
        instr!("LD B, E", 1, 4, Cpu::op_ld_b_e),
        instr!("LD B, H", 1, 4, Cpu::op_ld_b_h),
        instr!("LD B, L", 1, 4, Cpu::op_ld_b_l),
        instr!("LD B, (HL)", 1, 8, Cpu::op_ld_b_hlp),
        instr!("LD B, A", 1, 4, Cpu::op_ld_b_a),
        // 0x48 - 0x4F : LD C, r
        instr!("LD C, B", 1, 4, Cpu::op_ld_c_b),
        instr!("LD C, C", 1, 4, Cpu::op_ld_c_c),
        instr!("LD C, D", 1, 4, Cpu::op_ld_c_d),
        instr!("LD C, E", 1, 4, Cpu::op_ld_c_e),
        instr!("LD C, H", 1, 4, Cpu::op_ld_c_h),
        instr!("LD C, L", 1, 4, Cpu::op_ld_c_l),
        instr!("LD C, (HL)", 1, 8, Cpu::op_ld_c_hlp),
        instr!("LD C, A", 1, 4, Cpu::op_ld_c_a),
        // 0x50 - 0x57 : LD D, r
        instr!("LD D, B", 1, 4, Cpu::op_ld_d_b),
        instr!("LD D, C", 1, 4, Cpu::op_ld_d_c),
        instr!("LD D, D", 1, 4, Cpu::op_ld_d_d),
        instr!("LD D, E", 1, 4, Cpu::op_ld_d_e),
        instr!("LD D, H", 1, 4, Cpu::op_ld_d_h),
        instr!("LD D, L", 1, 4, Cpu::op_ld_d_l),
        instr!("LD D, (HL)", 1, 8, Cpu::op_ld_d_hlp),
        instr!("LD D, A", 1, 4, Cpu::op_ld_d_a),
        // 0x58 - 0x5F : LD E, r
        instr!("LD E, B", 1, 4, Cpu::op_ld_e_b),
        instr!("LD E, C", 1, 4, Cpu::op_ld_e_c),
        instr!("LD E, D", 1, 4, Cpu::op_ld_e_d),
        instr!("LD E, E", 1, 4, Cpu::op_ld_e_e),
        instr!("LD E, H", 1, 4, Cpu::op_ld_e_h),
        instr!("LD E, L", 1, 4, Cpu::op_ld_e_l),
        instr!("LD E, (HL)", 1, 8, Cpu::op_ld_e_hlp),
        instr!("LD E, A", 1, 4, Cpu::op_ld_e_a),
        // 0x60 - 0x67 : LD H, r
        instr!("LD H, B", 1, 4, Cpu::op_ld_h_b),
        instr!("LD H, C", 1, 4, Cpu::op_ld_h_c),
        instr!("LD H, D", 1, 4, Cpu::op_ld_h_d),
        instr!("LD H, E", 1, 4, Cpu::op_ld_h_e),
        instr!("LD H, H", 1, 4, Cpu::op_ld_h_h),
        instr!("LD H, L", 1, 4, Cpu::op_ld_h_l),
        instr!("LD H, (HL)", 1, 8, Cpu::op_ld_h_hlp),
        instr!("LD H, A", 1, 4, Cpu::op_ld_h_a),
        // 0x68 - 0x6F : LD L, r
        instr!("LD L, B", 1, 4, Cpu::op_ld_l_b),
        instr!("LD L, C", 1, 4, Cpu::op_ld_l_c),
        instr!("LD L, D", 1, 4, Cpu::op_ld_l_d),
        instr!("LD L, E", 1, 4, Cpu::op_ld_l_e),
        instr!("LD L, H", 1, 4, Cpu::op_ld_l_h),
        instr!("LD L, L", 1, 4, Cpu::op_ld_l_l),
        instr!("LD L, (HL)", 1, 8, Cpu::op_ld_l_hlp),
        instr!("LD L, A", 1, 4, Cpu::op_ld_l_a),
        // 0x70 - 0x77 : LD (HL), r
        instr!("LD (HL), B", 1, 8, Cpu::op_ld_hlp_b),
        instr!("LD (HL), C", 1, 8, Cpu::op_ld_hlp_c),
        instr!("LD (HL), D", 1, 8, Cpu::op_ld_hlp_d),
        instr!("LD (HL), E", 1, 8, Cpu::op_ld_hlp_e),
        instr!("LD (HL), H", 1, 8, Cpu::op_ld_hlp_h),
        instr!("LD (HL), L", 1, 8, Cpu::op_ld_hlp_l),
        instr!("HALT", 1, 4, Cpu::op_halt),             // 76 HALT
        instr!("LD (HL), A", 1, 8, Cpu::op_ld_hlp_a),
        // 0x78 - 0x7F : LD A, r
        instr!("LD A, B", 1, 4, Cpu::op_ld_a_b),
        instr!("LD A, C", 1, 4, Cpu::op_ld_a_c),
        instr!("LD A, D", 1, 4, Cpu::op_ld_a_d),
        instr!("LD A, E", 1, 4, Cpu::op_ld_a_e),
        instr!("LD A, H", 1, 4, Cpu::op_ld_a_h),
        instr!("LD A, L", 1, 4, Cpu::op_ld_a_l),
        instr!("LD A, (HL)", 1, 8, Cpu::op_ld_a_hlp),
        instr!("LD A, A", 1, 4, Cpu::op_ld_a_a),

        // --- 0x80..0xBF: ALU A, r ---
        // 0x80 - 0x87 : ADD A, r
        instr!("ADD A, B", 1, 4, Cpu::op_add_a_b),
        instr!("ADD A, C", 1, 4, Cpu::op_add_a_c),
        instr!("ADD A, D", 1, 4, Cpu::op_add_a_d),
        instr!("ADD A, E", 1, 4, Cpu::op_add_a_e),
        instr!("ADD A, H", 1, 4, Cpu::op_add_a_h),
        instr!("ADD A, L", 1, 4, Cpu::op_add_a_l),
        instr!("ADD A, (HL)", 1, 8, Cpu::op_add_a_hlp),
        instr!("ADD A, A", 1, 4, Cpu::op_add_a_a),
        // 0x88 - 0x8F : ADC A, r
        instr!("ADC A, B", 1, 4, Cpu::op_adc_a_b),
        instr!("ADC A, C", 1, 4, Cpu::op_adc_a_c),
        instr!("ADC A, D", 1, 4, Cpu::op_adc_a_d),
        instr!("ADC A, E", 1, 4, Cpu::op_adc_a_e),
        instr!("ADC A, H", 1, 4, Cpu::op_adc_a_h),
        instr!("ADC A, L", 1, 4, Cpu::op_adc_a_l),
        instr!("ADC A, (HL)", 1, 8, Cpu::op_adc_a_hlp),
        instr!("ADC A, A", 1, 4, Cpu::op_adc_a_a),
        // 0x90 - 0x97 : SUB A, r
        instr!("SUB A, B", 1, 4, Cpu::op_sub_a_b),
        instr!("SUB A, C", 1, 4, Cpu::op_sub_a_c),
        instr!("SUB A, D", 1, 4, Cpu::op_sub_a_d),
        instr!("SUB A, E", 1, 4, Cpu::op_sub_a_e),
        instr!("SUB A, H", 1, 4, Cpu::op_sub_a_h),
        instr!("SUB A, L", 1, 4, Cpu::op_sub_a_l),
        instr!("SUB A, (HL)", 1, 8, Cpu::op_sub_a_hlp),
        instr!("SUB A, A", 1, 4, Cpu::op_sub_a_a),
        // 0x98 - 0x9F : SBC A, r
        instr!("SBC A, B", 1, 4, Cpu::op_sbc_a_b),
        instr!("SBC A, C", 1, 4, Cpu::op_sbc_a_c),
        instr!("SBC A, D", 1, 4, Cpu::op_sbc_a_d),
        instr!("SBC A, E", 1, 4, Cpu::op_sbc_a_e),
        instr!("SBC A, H", 1, 4, Cpu::op_sbc_a_h),
        instr!("SBC A, L", 1, 4, Cpu::op_sbc_a_l),
        instr!("SBC A, (HL)", 1, 8, Cpu::op_sbc_a_hlp),
        instr!("SBC A, A", 1, 4, Cpu::op_sbc_a_a),
        // 0xA0 - 0xA7 : AND A, r
        instr!("AND A, B", 1, 4, Cpu::op_and_a_b),
        instr!("AND A, C", 1, 4, Cpu::op_and_a_c),
        instr!("AND A, D", 1, 4, Cpu::op_and_a_d),
        instr!("AND A, E", 1, 4, Cpu::op_and_a_e),
        instr!("AND A, H", 1, 4, Cpu::op_and_a_h),
        instr!("AND A, L", 1, 4, Cpu::op_and_a_l),
        instr!("AND A, (HL)", 1, 8, Cpu::op_and_a_hlp),
        instr!("AND A, A", 1, 4, Cpu::op_and_a_a),
        // 0xA8 - 0xAF : XOR A, r
        instr!("XOR A, B", 1, 4, Cpu::op_xor_a_b),
        instr!("XOR A, C", 1, 4, Cpu::op_xor_a_c),
        instr!("XOR A, D", 1, 4, Cpu::op_xor_a_d),
        instr!("XOR A, E", 1, 4, Cpu::op_xor_a_e),
        instr!("XOR A, H", 1, 4, Cpu::op_xor_a_h),
        instr!("XOR A, L", 1, 4, Cpu::op_xor_a_l),
        instr!("XOR A, (HL)", 1, 8, Cpu::op_xor_a_hlp),
        instr!("XOR A, A", 1, 4, Cpu::op_xor_a_a),
        // 0xB0 - 0xB7 : OR A, r
        instr!("OR A, B", 1, 4, Cpu::op_or_a_b),
        instr!("OR A, C", 1, 4, Cpu::op_or_a_c),
        instr!("OR A, D", 1, 4, Cpu::op_or_a_d),
        instr!("OR A, E", 1, 4, Cpu::op_or_a_e),
        instr!("OR A, H", 1, 4, Cpu::op_or_a_h),
        instr!("OR A, L", 1, 4, Cpu::op_or_a_l),
        instr!("OR A, (HL)", 1, 8, Cpu::op_or_a_hlp),
        instr!("OR A, A", 1, 4, Cpu::op_or_a_a),
        // 0xB8 - 0xBF : CP A, r
        instr!("CP A, B", 1, 4, Cpu::op_cp_a_b),
        instr!("CP A, C", 1, 4, Cpu::op_cp_a_c),
        instr!("CP A, D", 1, 4, Cpu::op_cp_a_d),
        instr!("CP A, E", 1, 4, Cpu::op_cp_a_e),
        instr!("CP A, H", 1, 4, Cpu::op_cp_a_h),
        instr!("CP A, L", 1, 4, Cpu::op_cp_a_l),
        instr!("CP A, (HL)", 1, 8, Cpu::op_cp_a_hlp),
        instr!("CP A, A", 1, 4, Cpu::op_cp_a_a),

        // --- 0xC0 --- RET NZ (8 cycles if no return, 20 if return)
        instr!("RET NZ", 1, 8, Cpu::op_ret_nz),         // C0 RET NZ
        instr!("POP BC", 1, 12, Cpu::op_pop_bc),        // C1 POP BC
        // --- 0xC2 --- JP NZ,a16 (12 cycles if no jump, 16 if jump)
        instr!("JP NZ, a16", 3, 12, Cpu::op_jp_nz_a16), // C2 JP NZ, a16
        instr!("JP a16", 3, 16, Cpu::op_jp_a16),        // C3 JP a16
        // --- 0xC4 --- CALL NZ,a16 (12 cycles if no call, 24 if call)
        instr!("CALL NZ, a16", 3, 12, Cpu::op_call_nz_a16), // C4 CALL NZ, a16
        instr!("PUSH BC", 1, 16, Cpu::op_push_bc),      // C5 PUSH BC
        instr!("ADD A, d8", 2, 8, Cpu::op_add_a_d8),    // C6 ADD A, d8
        instr!("RST 00H", 1, 16, Cpu::op_rst_00h),      // C7 RST 00H
        // --- 0xC8 --- RET Z (8 cycles if no return, 20 if return)
        instr!("RET Z", 1, 8, Cpu::op_ret_z),           // C8 RET Z
        instr!("RET", 1, 16, Cpu::op_ret),              // C9 RET
        // --- 0xCA --- JP Z,a16 (12 cycles if no jump, 16 if jump)
        instr!("JP Z, a16", 3, 12, Cpu::op_jp_z_a16),   // CA JP Z, a16
        instr!("PREFIX CB", 1, 4, Cpu::op_prefix_cb),   // CB PREFIX CB
        // --- 0xCC --- CALL Z,a16 (12 cycles if no call, 24 if call)
        instr!("CALL Z, a16", 3, 12, Cpu::op_call_z_a16), // CC CALL Z, a16
        instr!("CALL a16", 3, 24, Cpu::op_call_a16),    // CD CALL a16
        instr!("ADC A, d8", 2, 8, Cpu::op_adc_a_d8),    // CE ADC A, d8
        instr!("RST 08H", 1, 16, Cpu::op_rst_08h),      // CF RST 08H

        // --- 0xD0 --- RET NC (8 cycles if no return, 20 if return)
        instr!("RET NC", 1, 8, Cpu::op_ret_nc),         // D0 RET NC
        instr!("POP DE", 1, 12, Cpu::op_pop_de),        // D1 POP DE
        // --- 0xD2 --- JP NC,a16 (12 cycles if no jump, 16 if jump)
        instr!("JP NC, a16", 3, 12, Cpu::op_jp_nc_a16), // D2 JP NC, a16
        invalid!(),                                     // D3 Invalid
        // --- 0xD4 --- CALL NC,a16 (12 cycles if no call, 24 if call)
        instr!("CALL NC, a16", 3, 12, Cpu::op_call_nc_a16), // D4 CALL NC, a16
        instr!("PUSH DE", 1, 16, Cpu::op_push_de),      // D5 PUSH DE
        instr!("SUB A, d8", 2, 8, Cpu::op_sub_a_d8),    // D6 SUB A, d8
        instr!("RST 10H", 1, 16, Cpu::op_rst_10h),      // D7 RST 10H
        // --- 0xD8 --- RET C (8 cycles if no return, 20 if return)
        instr!("RET C", 1, 8, Cpu::op_ret_c),           // D8 RET C
        instr!("RETI", 1, 16, Cpu::op_reti),            // D9 RETI
        // --- 0xDA --- JP C,a16 (12 cycles if no jump, 16 if jump)
        instr!("JP C, a16", 3, 12, Cpu::op_jp_c_a16),   // DA JP C, a16
        invalid!(),                                     // DB Invalid
        // --- 0xDC --- CALL C,a16 (12 cycles if no call, 24 if call)
        instr!("CALL C, a16", 3, 12, Cpu::op_call_c_a16), // DC CALL C, a16
        invalid!(),                                     // DD Invalid
        instr!("SBC A, d8", 2, 8, Cpu::op_sbc_a_d8),    // DE SBC A, d8
        instr!("RST 18H", 1, 16, Cpu::op_rst_18h),      // DF RST 18H

        // --- 0xE0 ---
        instr!("LDH (a8), A", 2, 12, Cpu::op_ldh_a8_a), // E0 LDH (a8), A
        instr!("POP HL", 1, 12, Cpu::op_pop_hl),        // E1 POP HL
        instr!("LD (C), A", 1, 8, Cpu::op_ld_cp_a),     // E2 LD (C), A ; Note: Mnemonic uses C not (C)
        invalid!(),                                     // E3 Invalid
        invalid!(),                                     // E4 Invalid
        instr!("PUSH HL", 1, 16, Cpu::op_push_hl),      // E5 PUSH HL
        instr!("AND A, d8", 2, 8, Cpu::op_and_a_d8),    // E6 AND A, d8
        instr!("RST 20H", 1, 16, Cpu::op_rst_20h),      // E7 RST 20H
        instr!("ADD SP, r8", 2, 16, Cpu::op_add_sp_r8), // E8 ADD SP, r8
        instr!("JP HL", 1, 4, Cpu::op_jp_hl),           // E9 JP HL ; Mnemonic sometimes (HL)
        instr!("LD (a16), A", 3, 16, Cpu::op_ld_a16_a), // EA LD (a16), A
        invalid!(),                                     // EB Invalid
        invalid!(),                                     // EC Invalid
        invalid!(),                                     // ED Invalid
        instr!("XOR A, d8", 2, 8, Cpu::op_xor_a_d8),    // EE XOR A, d8
        instr!("RST 28H", 1, 16, Cpu::op_rst_28h),      // EF RST 28H

        // --- 0xF0 ---
        instr!("LDH A, (a8)", 2, 12, Cpu::op_ldh_a_a8), // F0 LDH A, (a8)
        instr!("POP AF", 1, 12, Cpu::op_pop_af),        // F1 POP AF
        instr!("LD A, (C)", 1, 8, Cpu::op_ld_a_cp),     // F2 LD A, (C) ; Note: Mnemonic uses C not (C)
        instr!("DI", 1, 4, Cpu::op_di),                 // F3 DI
        invalid!(),                                     // F4 Invalid
        instr!("PUSH AF", 1, 16, Cpu::op_push_af),      // F5 PUSH AF
        instr!("OR A, d8", 2, 8, Cpu::op_or_a_d8),      // F6 OR A, d8
        instr!("RST 30H", 1, 16, Cpu::op_rst_30h),      // F7 RST 30H
        instr!("LD HL, SP+r8", 2, 12, Cpu::op_ld_hl_sp_r8), // F8 LD HL, SP+r8
        instr!("LD SP, HL", 1, 8, Cpu::op_ld_sp_hl),    // F9 LD SP, HL
        instr!("LD A, (a16)", 3, 16, Cpu::op_ld_a_a16), // FA LD A, (a16)
        instr!("EI", 1, 4, Cpu::op_ei),                 // FB EI
        invalid!(),                                     // FC Invalid
        invalid!(),                                     // FD Invalid
        instr!("CP A, d8", 2, 8, Cpu::op_cp_a_d8),      // FE CP A, d8
        instr!("RST 38H", 1, 16, Cpu::op_rst_38h),      // FF RST 38H
    ];

    // CB-prefixed instruction table (0x00 - 0xFF)
    pub static ref CB_INSTRUCTIONS: [Instruction; 256] = [
        // --- 0x00-0x3F: Rotates and Shifts ---
        // RLC r (Cycles: 8 reg, 16 (HL))
        instr!("RLC B", 1, 8, Cpu::cb_rlc_b), instr!("RLC C", 1, 8, Cpu::cb_rlc_c),
        instr!("RLC D", 1, 8, Cpu::cb_rlc_d), instr!("RLC E", 1, 8, Cpu::cb_rlc_e),
        instr!("RLC H", 1, 8, Cpu::cb_rlc_h), instr!("RLC L", 1, 8, Cpu::cb_rlc_l),
        instr!("RLC (HL)", 1, 16, Cpu::cb_rlc_hlp), instr!("RLC A", 1, 8, Cpu::cb_rlc_a),
        // RRC r
        instr!("RRC B", 1, 8, Cpu::cb_rrc_b), instr!("RRC C", 1, 8, Cpu::cb_rrc_c),
        instr!("RRC D", 1, 8, Cpu::cb_rrc_d), instr!("RRC E", 1, 8, Cpu::cb_rrc_e),
        instr!("RRC H", 1, 8, Cpu::cb_rrc_h), instr!("RRC L", 1, 8, Cpu::cb_rrc_l),
        instr!("RRC (HL)", 1, 16, Cpu::cb_rrc_hlp), instr!("RRC A", 1, 8, Cpu::cb_rrc_a),
        // RL r
        instr!("RL B", 1, 8, Cpu::cb_rl_b), instr!("RL C", 1, 8, Cpu::cb_rl_c),
        instr!("RL D", 1, 8, Cpu::cb_rl_d), instr!("RL E", 1, 8, Cpu::cb_rl_e),
        instr!("RL H", 1, 8, Cpu::cb_rl_h), instr!("RL L", 1, 8, Cpu::cb_rl_l),
        instr!("RL (HL)", 1, 16, Cpu::cb_rl_hlp), instr!("RL A", 1, 8, Cpu::cb_rl_a),
        // RR r
        instr!("RR B", 1, 8, Cpu::cb_rr_b), instr!("RR C", 1, 8, Cpu::cb_rr_c),
        instr!("RR D", 1, 8, Cpu::cb_rr_d), instr!("RR E", 1, 8, Cpu::cb_rr_e),
        instr!("RR H", 1, 8, Cpu::cb_rr_h), instr!("RR L", 1, 8, Cpu::cb_rr_l),
        instr!("RR (HL)", 1, 16, Cpu::cb_rr_hlp), instr!("RR A", 1, 8, Cpu::cb_rr_a),
        // SLA r
        instr!("SLA B", 1, 8, Cpu::cb_sla_b), instr!("SLA C", 1, 8, Cpu::cb_sla_c),
        instr!("SLA D", 1, 8, Cpu::cb_sla_d), instr!("SLA E", 1, 8, Cpu::cb_sla_e),
        instr!("SLA H", 1, 8, Cpu::cb_sla_h), instr!("SLA L", 1, 8, Cpu::cb_sla_l),
        instr!("SLA (HL)", 1, 16, Cpu::cb_sla_hlp), instr!("SLA A", 1, 8, Cpu::cb_sla_a),
        // SRA r
        instr!("SRA B", 1, 8, Cpu::cb_sra_b), instr!("SRA C", 1, 8, Cpu::cb_sra_c),
        instr!("SRA D", 1, 8, Cpu::cb_sra_d), instr!("SRA E", 1, 8, Cpu::cb_sra_e),
        instr!("SRA H", 1, 8, Cpu::cb_sra_h), instr!("SRA L", 1, 8, Cpu::cb_sra_l),
        instr!("SRA (HL)", 1, 16, Cpu::cb_sra_hlp), instr!("SRA A", 1, 8, Cpu::cb_sra_a),
        // SWAP r
        instr!("SWAP B", 1, 8, Cpu::cb_swap_b), instr!("SWAP C", 1, 8, Cpu::cb_swap_c),
        instr!("SWAP D", 1, 8, Cpu::cb_swap_d), instr!("SWAP E", 1, 8, Cpu::cb_swap_e),
        instr!("SWAP H", 1, 8, Cpu::cb_swap_h), instr!("SWAP L", 1, 8, Cpu::cb_swap_l),
        instr!("SWAP (HL)", 1, 16, Cpu::cb_swap_hlp), instr!("SWAP A", 1, 8, Cpu::cb_swap_a),
        // SRL r
        instr!("SRL B", 1, 8, Cpu::cb_srl_b), instr!("SRL C", 1, 8, Cpu::cb_srl_c),
        instr!("SRL D", 1, 8, Cpu::cb_srl_d), instr!("SRL E", 1, 8, Cpu::cb_srl_e),
        instr!("SRL H", 1, 8, Cpu::cb_srl_h), instr!("SRL L", 1, 8, Cpu::cb_srl_l),
        instr!("SRL (HL)", 1, 16, Cpu::cb_srl_hlp), instr!("SRL A", 1, 8, Cpu::cb_srl_a),

        // --- 0x40-0x7F: BIT b, r --- (Cycles: 8 reg, 12 (HL))
        // BIT 0, r
        instr!("BIT 0, B", 1, 8, Cpu::cb_bit_0_b), instr!("BIT 0, C", 1, 8, Cpu::cb_bit_0_c),
        instr!("BIT 0, D", 1, 8, Cpu::cb_bit_0_d), instr!("BIT 0, E", 1, 8, Cpu::cb_bit_0_e),
        instr!("BIT 0, H", 1, 8, Cpu::cb_bit_0_h), instr!("BIT 0, L", 1, 8, Cpu::cb_bit_0_l),
        instr!("BIT 0, (HL)", 1, 12, Cpu::cb_bit_0_hlp), instr!("BIT 0, A", 1, 8, Cpu::cb_bit_0_a),
        // BIT 1, r
        instr!("BIT 1, B", 1, 8, Cpu::cb_bit_1_b), instr!("BIT 1, C", 1, 8, Cpu::cb_bit_1_c),
        instr!("BIT 1, D", 1, 8, Cpu::cb_bit_1_d), instr!("BIT 1, E", 1, 8, Cpu::cb_bit_1_e),
        instr!("BIT 1, H", 1, 8, Cpu::cb_bit_1_h), instr!("BIT 1, L", 1, 8, Cpu::cb_bit_1_l),
        instr!("BIT 1, (HL)", 1, 12, Cpu::cb_bit_1_hlp), instr!("BIT 1, A", 1, 8, Cpu::cb_bit_1_a),
        // BIT 2, r
        instr!("BIT 2, B", 1, 8, Cpu::cb_bit_2_b), instr!("BIT 2, C", 1, 8, Cpu::cb_bit_2_c),
        instr!("BIT 2, D", 1, 8, Cpu::cb_bit_2_d), instr!("BIT 2, E", 1, 8, Cpu::cb_bit_2_e),
        instr!("BIT 2, H", 1, 8, Cpu::cb_bit_2_h), instr!("BIT 2, L", 1, 8, Cpu::cb_bit_2_l),
        instr!("BIT 2, (HL)", 1, 12, Cpu::cb_bit_2_hlp), instr!("BIT 2, A", 1, 8, Cpu::cb_bit_2_a),
        // BIT 3, r
        instr!("BIT 3, B", 1, 8, Cpu::cb_bit_3_b), instr!("BIT 3, C", 1, 8, Cpu::cb_bit_3_c),
        instr!("BIT 3, D", 1, 8, Cpu::cb_bit_3_d), instr!("BIT 3, E", 1, 8, Cpu::cb_bit_3_e),
        instr!("BIT 3, H", 1, 8, Cpu::cb_bit_3_h), instr!("BIT 3, L", 1, 8, Cpu::cb_bit_3_l),
        instr!("BIT 3, (HL)", 1, 12, Cpu::cb_bit_3_hlp), instr!("BIT 3, A", 1, 8, Cpu::cb_bit_3_a),
        // BIT 4, r
        instr!("BIT 4, B", 1, 8, Cpu::cb_bit_4_b), instr!("BIT 4, C", 1, 8, Cpu::cb_bit_4_c),
        instr!("BIT 4, D", 1, 8, Cpu::cb_bit_4_d), instr!("BIT 4, E", 1, 8, Cpu::cb_bit_4_e),
        instr!("BIT 4, H", 1, 8, Cpu::cb_bit_4_h), instr!("BIT 4, L", 1, 8, Cpu::cb_bit_4_l),
        instr!("BIT 4, (HL)", 1, 12, Cpu::cb_bit_4_hlp), instr!("BIT 4, A", 1, 8, Cpu::cb_bit_4_a),
        // BIT 5, r
        instr!("BIT 5, B", 1, 8, Cpu::cb_bit_5_b), instr!("BIT 5, C", 1, 8, Cpu::cb_bit_5_c),
        instr!("BIT 5, D", 1, 8, Cpu::cb_bit_5_d), instr!("BIT 5, E", 1, 8, Cpu::cb_bit_5_e),
        instr!("BIT 5, H", 1, 8, Cpu::cb_bit_5_h), instr!("BIT 5, L", 1, 8, Cpu::cb_bit_5_l),
        instr!("BIT 5, (HL)", 1, 12, Cpu::cb_bit_5_hlp), instr!("BIT 5, A", 1, 8, Cpu::cb_bit_5_a),
        // BIT 6, r
        instr!("BIT 6, B", 1, 8, Cpu::cb_bit_6_b), instr!("BIT 6, C", 1, 8, Cpu::cb_bit_6_c),
        instr!("BIT 6, D", 1, 8, Cpu::cb_bit_6_d), instr!("BIT 6, E", 1, 8, Cpu::cb_bit_6_e),
        instr!("BIT 6, H", 1, 8, Cpu::cb_bit_6_h), instr!("BIT 6, L", 1, 8, Cpu::cb_bit_6_l),
        instr!("BIT 6, (HL)", 1, 12, Cpu::cb_bit_6_hlp), instr!("BIT 6, A", 1, 8, Cpu::cb_bit_6_a),
        // BIT 7, r
        instr!("BIT 7, B", 1, 8, Cpu::cb_bit_7_b), instr!("BIT 7, C", 1, 8, Cpu::cb_bit_7_c),
        instr!("BIT 7, D", 1, 8, Cpu::cb_bit_7_d), instr!("BIT 7, E", 1, 8, Cpu::cb_bit_7_e),
        instr!("BIT 7, H", 1, 8, Cpu::cb_bit_7_h), instr!("BIT 7, L", 1, 8, Cpu::cb_bit_7_l),
        instr!("BIT 7, (HL)", 1, 12, Cpu::cb_bit_7_hlp), instr!("BIT 7, A", 1, 8, Cpu::cb_bit_7_a),

        // --- 0x80-0xBF: RES b, r --- (Cycles: 8 reg, 16 (HL))
        // RES 0, r
        instr!("RES 0, B", 1, 8, Cpu::cb_res_0_b), instr!("RES 0, C", 1, 8, Cpu::cb_res_0_c),
        instr!("RES 0, D", 1, 8, Cpu::cb_res_0_d), instr!("RES 0, E", 1, 8, Cpu::cb_res_0_e),
        instr!("RES 0, H", 1, 8, Cpu::cb_res_0_h), instr!("RES 0, L", 1, 8, Cpu::cb_res_0_l),
        instr!("RES 0, (HL)", 1, 16, Cpu::cb_res_0_hlp), instr!("RES 0, A", 1, 8, Cpu::cb_res_0_a),
        // RES 1, r
        instr!("RES 1, B", 1, 8, Cpu::cb_res_1_b), instr!("RES 1, C", 1, 8, Cpu::cb_res_1_c),
        instr!("RES 1, D", 1, 8, Cpu::cb_res_1_d), instr!("RES 1, E", 1, 8, Cpu::cb_res_1_e),
        instr!("RES 1, H", 1, 8, Cpu::cb_res_1_h), instr!("RES 1, L", 1, 8, Cpu::cb_res_1_l),
        instr!("RES 1, (HL)", 1, 16, Cpu::cb_res_1_hlp), instr!("RES 1, A", 1, 8, Cpu::cb_res_1_a),
        // RES 2, r
        instr!("RES 2, B", 1, 8, Cpu::cb_res_2_b), instr!("RES 2, C", 1, 8, Cpu::cb_res_2_c),
        instr!("RES 2, D", 1, 8, Cpu::cb_res_2_d), instr!("RES 2, E", 1, 8, Cpu::cb_res_2_e),
        instr!("RES 2, H", 1, 8, Cpu::cb_res_2_h), instr!("RES 2, L", 1, 8, Cpu::cb_res_2_l),
        instr!("RES 2, (HL)", 1, 16, Cpu::cb_res_2_hlp), instr!("RES 2, A", 1, 8, Cpu::cb_res_2_a),
        // RES 3, r
        instr!("RES 3, B", 1, 8, Cpu::cb_res_3_b), instr!("RES 3, C", 1, 8, Cpu::cb_res_3_c),
        instr!("RES 3, D", 1, 8, Cpu::cb_res_3_d), instr!("RES 3, E", 1, 8, Cpu::cb_res_3_e),
        instr!("RES 3, H", 1, 8, Cpu::cb_res_3_h), instr!("RES 3, L", 1, 8, Cpu::cb_res_3_l),
        instr!("RES 3, (HL)", 1, 16, Cpu::cb_res_3_hlp), instr!("RES 3, A", 1, 8, Cpu::cb_res_3_a),
        // RES 4, r
        instr!("RES 4, B", 1, 8, Cpu::cb_res_4_b), instr!("RES 4, C", 1, 8, Cpu::cb_res_4_c),
        instr!("RES 4, D", 1, 8, Cpu::cb_res_4_d), instr!("RES 4, E", 1, 8, Cpu::cb_res_4_e),
        instr!("RES 4, H", 1, 8, Cpu::cb_res_4_h), instr!("RES 4, L", 1, 8, Cpu::cb_res_4_l),
        instr!("RES 4, (HL)", 1, 16, Cpu::cb_res_4_hlp), instr!("RES 4, A", 1, 8, Cpu::cb_res_4_a),
        // RES 5, r
        instr!("RES 5, B", 1, 8, Cpu::cb_res_5_b), instr!("RES 5, C", 1, 8, Cpu::cb_res_5_c),
        instr!("RES 5, D", 1, 8, Cpu::cb_res_5_d), instr!("RES 5, E", 1, 8, Cpu::cb_res_5_e),
        instr!("RES 5, H", 1, 8, Cpu::cb_res_5_h), instr!("RES 5, L", 1, 8, Cpu::cb_res_5_l),
        instr!("RES 5, (HL)", 1, 16, Cpu::cb_res_5_hlp), instr!("RES 5, A", 1, 8, Cpu::cb_res_5_a),
        // RES 6, r
        instr!("RES 6, B", 1, 8, Cpu::cb_res_6_b), instr!("RES 6, C", 1, 8, Cpu::cb_res_6_c),
        instr!("RES 6, D", 1, 8, Cpu::cb_res_6_d), instr!("RES 6, E", 1, 8, Cpu::cb_res_6_e),
        instr!("RES 6, H", 1, 8, Cpu::cb_res_6_h), instr!("RES 6, L", 1, 8, Cpu::cb_res_6_l),
        instr!("RES 6, (HL)", 1, 16, Cpu::cb_res_6_hlp), instr!("RES 6, A", 1, 8, Cpu::cb_res_6_a),
        // RES 7, r
        instr!("RES 7, B", 1, 8, Cpu::cb_res_7_b), instr!("RES 7, C", 1, 8, Cpu::cb_res_7_c),
        instr!("RES 7, D", 1, 8, Cpu::cb_res_7_d), instr!("RES 7, E", 1, 8, Cpu::cb_res_7_e),
        instr!("RES 7, H", 1, 8, Cpu::cb_res_7_h), instr!("RES 7, L", 1, 8, Cpu::cb_res_7_l),
        instr!("RES 7, (HL)", 1, 16, Cpu::cb_res_7_hlp), instr!("RES 7, A", 1, 8, Cpu::cb_res_7_a),

        // --- 0xC0-0xFF: SET b, r --- (Cycles: 8 reg, 16 (HL))
        // SET 0, r
        instr!("SET 0, B", 1, 8, Cpu::cb_set_0_b), instr!("SET 0, C", 1, 8, Cpu::cb_set_0_c),
        instr!("SET 0, D", 1, 8, Cpu::cb_set_0_d), instr!("SET 0, E", 1, 8, Cpu::cb_set_0_e),
        instr!("SET 0, H", 1, 8, Cpu::cb_set_0_h), instr!("SET 0, L", 1, 8, Cpu::cb_set_0_l),
        instr!("SET 0, (HL)", 1, 16, Cpu::cb_set_0_hlp), instr!("SET 0, A", 1, 8, Cpu::cb_set_0_a),
         // SET 1, r
        instr!("SET 1, B", 1, 8, Cpu::cb_set_1_b), instr!("SET 1, C", 1, 8, Cpu::cb_set_1_c),
        instr!("SET 1, D", 1, 8, Cpu::cb_set_1_d), instr!("SET 1, E", 1, 8, Cpu::cb_set_1_e),
        instr!("SET 1, H", 1, 8, Cpu::cb_set_1_h), instr!("SET 1, L", 1, 8, Cpu::cb_set_1_l),
        instr!("SET 1, (HL)", 1, 16, Cpu::cb_set_1_hlp), instr!("SET 1, A", 1, 8, Cpu::cb_set_1_a),
        // SET 2, r
        instr!("SET 2, B", 1, 8, Cpu::cb_set_2_b), instr!("SET 2, C", 1, 8, Cpu::cb_set_2_c),
        instr!("SET 2, D", 1, 8, Cpu::cb_set_2_d), instr!("SET 2, E", 1, 8, Cpu::cb_set_2_e),
        instr!("SET 2, H", 1, 8, Cpu::cb_set_2_h), instr!("SET 2, L", 1, 8, Cpu::cb_set_2_l),
        instr!("SET 2, (HL)", 1, 16, Cpu::cb_set_2_hlp), instr!("SET 2, A", 1, 8, Cpu::cb_set_2_a),
        // SET 3, r
        instr!("SET 3, B", 1, 8, Cpu::cb_set_3_b), instr!("SET 3, C", 1, 8, Cpu::cb_set_3_c),
        instr!("SET 3, D", 1, 8, Cpu::cb_set_3_d), instr!("SET 3, E", 1, 8, Cpu::cb_set_3_e),
        instr!("SET 3, H", 1, 8, Cpu::cb_set_3_h), instr!("SET 3, L", 1, 8, Cpu::cb_set_3_l),
        instr!("SET 3, (HL)", 1, 16, Cpu::cb_set_3_hlp), instr!("SET 3, A", 1, 8, Cpu::cb_set_3_a),
        // SET 4, r
        instr!("SET 4, B", 1, 8, Cpu::cb_set_4_b), instr!("SET 4, C", 1, 8, Cpu::cb_set_4_c),
        instr!("SET 4, D", 1, 8, Cpu::cb_set_4_d), instr!("SET 4, E", 1, 8, Cpu::cb_set_4_e),
        instr!("SET 4, H", 1, 8, Cpu::cb_set_4_h), instr!("SET 4, L", 1, 8, Cpu::cb_set_4_l),
        instr!("SET 4, (HL)", 1, 16, Cpu::cb_set_4_hlp), instr!("SET 4, A", 1, 8, Cpu::cb_set_4_a),
        // SET 5, r
        instr!("SET 5, B", 1, 8, Cpu::cb_set_5_b), instr!("SET 5, C", 1, 8, Cpu::cb_set_5_c),
        instr!("SET 5, D", 1, 8, Cpu::cb_set_5_d), instr!("SET 5, E", 1, 8, Cpu::cb_set_5_e),
        instr!("SET 5, H", 1, 8, Cpu::cb_set_5_h), instr!("SET 5, L", 1, 8, Cpu::cb_set_5_l),
        instr!("SET 5, (HL)", 1, 16, Cpu::cb_set_5_hlp), instr!("SET 5, A", 1, 8, Cpu::cb_set_5_a),
        // SET 6, r
        instr!("SET 6, B", 1, 8, Cpu::cb_set_6_b), instr!("SET 6, C", 1, 8, Cpu::cb_set_6_c),
        instr!("SET 6, D", 1, 8, Cpu::cb_set_6_d), instr!("SET 6, E", 1, 8, Cpu::cb_set_6_e),
        instr!("SET 6, H", 1, 8, Cpu::cb_set_6_h), instr!("SET 6, L", 1, 8, Cpu::cb_set_6_l),
        instr!("SET 6, (HL)", 1, 16, Cpu::cb_set_6_hlp), instr!("SET 6, A", 1, 8, Cpu::cb_set_6_a),
        // SET 7, r
        instr!("SET 7, B", 1, 8, Cpu::cb_set_7_b), instr!("SET 7, C", 1, 8, Cpu::cb_set_7_c),
        instr!("SET 7, D", 1, 8, Cpu::cb_set_7_d), instr!("SET 7, E", 1, 8, Cpu::cb_set_7_e),
        instr!("SET 7, H", 1, 8, Cpu::cb_set_7_h), instr!("SET 7, L", 1, 8, Cpu::cb_set_7_l),
        instr!("SET 7, (HL)", 1, 16, Cpu::cb_set_7_hlp), instr!("SET 7, A", 1, 8, Cpu::cb_set_7_a),
    ];
}
