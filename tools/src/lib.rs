#![recursion_limit = "160"] // 150 was too low in rust 1.15
use std::result;
mod operand;
use operand::Operand;
extern crate r68k_common;
use r68k_common::constants::*;
mod constants;
use constants::*;
#[macro_use]
extern crate pest;

pub mod memory;
pub mod assembler;
pub mod disassembler;
pub mod srecords;

use memory::Memory;

// type alias for exception handling
pub type Result<T> = result::Result<T, Exception>;
type OpcodeValidator = fn(u16, u16) -> bool;
type OperandDecoder = fn(u16, Size, u32, &Memory) -> Vec<Operand>;
type InstructionEncoder = fn(&OpcodeInstance, u16, u32, &mut Memory) -> u32;
type InstructionSelector = fn(&OpcodeInstance) -> bool;

#[derive(Debug)]
pub enum Exception {
     IllegalInstruction(u16, u32), // ir, pc
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
	Unsized, Byte, Word, Long
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Size::Unsized => write!(f, ""),
            Size::Byte => write!(f, ".B"),
            Size::Word => write!(f, ".W"),
            Size::Long => write!(f, ".L"),
        }
    }
}

// #[derive(Clone, Copy)]
pub struct OpcodeInfo<'a> {
    mask: u32,
    matching: u32,
    ea_mask: u16,
    size: Size,
    validator: OpcodeValidator,
    decoder: OperandDecoder,
    mnemonic: &'a str,
    encoder: InstructionEncoder,
    selector: InstructionSelector,
}
#[derive(Clone, Debug, PartialEq)]
pub struct OpcodeInstance<'a> {
    pub mnemonic: &'a str,
    pub size: Size,
    pub operands: Vec<Operand>,
}

use std::fmt;
impl<'a> fmt::Debug for OpcodeInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            _ => write!(f, "[some fn]"),
        }
    }
}
impl<'a> fmt::Display for OpcodeInstance<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.operands.len() {
            0 => write!(f, "{}{}", self.mnemonic, self.size),
            1 => write!(f, "{}{}\t{}", self.mnemonic, self.size, self.operands[0]),
            2 => write!(f, "{}{}\t{},{}", self.mnemonic, self.size, self.operands[0], self.operands[1]),
            _ => panic!("more than two operands {:?}", self)
        }
    }
}
impl<'a> OpcodeInstance<'a> {
    pub fn length(&self) -> u32 {
        1 + self.operands.iter().map(|op| op.extension_words()).fold(0, |i,j|i+j)
    }
}
macro_rules! instruction {
    ($mask:expr, $matching:expr, $ea_mask:expr, $size:expr, $mnemonic:expr, $decoder:ident) => (OpcodeInfo { mask: $mask, matching: $matching, size: $size, mnemonic: $mnemonic, validator: dissassembler::valid_ea, decoder: disassembler::$decoder, encoder: assembler::nop_encoder, selector: assembler::nop_selector, ea_mask: $ea_mask});
    ($mask:expr, $matching:expr, $ea_mask:expr, $size:expr, $mnemonic:expr, $validator:ident, $decoder:ident, $selector:ident, $encoder:ident) => (OpcodeInfo { mask: $mask, matching: $matching, size: $size, mnemonic: $mnemonic, validator: disassembler::$validator, decoder: disassembler::$decoder, encoder: assembler::$encoder, selector: assembler::$selector, ea_mask: $ea_mask})
}
fn generate<'a>() -> Vec<OpcodeInfo<'a>> {
    vec![
        instruction!(MASK_OUT_X_EA, OP_ADD | BYTE_SIZED | DEST_EA, EA_MEMORY_ALTERABLE, Size::Byte, "ADD", valid_ea, decode_dx_ea, is_dx_ea, encode_dx_ea),
        instruction!(MASK_OUT_X_EA, OP_ADD | BYTE_SIZED | DEST_DX, EA_ALL_EXCEPT_AN, Size::Byte, "ADD", valid_ea, decode_ea_dx, is_ea_dx, encode_ea_dx),
        instruction!(MASK_OUT_X_EA, OP_ADD | WORD_SIZED | DEST_EA, EA_MEMORY_ALTERABLE, Size::Word, "ADD", valid_ea, decode_dx_ea, is_dx_ea, encode_dx_ea),
        instruction!(MASK_OUT_X_EA, OP_ADD | WORD_SIZED | DEST_DX, EA_ALL, Size::Word, "ADD", valid_ea, decode_ea_dx, is_ea_dx, encode_ea_dx),
        instruction!(MASK_OUT_X_EA, OP_ADD | LONG_SIZED | DEST_EA, EA_MEMORY_ALTERABLE, Size::Long, "ADD", valid_ea, decode_dx_ea, is_dx_ea, encode_dx_ea),
        instruction!(MASK_OUT_X_EA, OP_ADD | LONG_SIZED | DEST_DX, EA_ALL, Size::Long, "ADD", valid_ea, decode_ea_dx, is_ea_dx, encode_ea_dx),
        instruction!(MASK_OUT_X_EA, OP_ADD | DEST_AX_WORD, EA_ALL, Size::Word, "ADDA", valid_ea, decode_ea_ax, is_ea_ax, encode_ea_ax),
        instruction!(MASK_OUT_X_EA, OP_ADD | DEST_AX_LONG, EA_ALL, Size::Long, "ADDA", valid_ea, decode_ea_ax, is_ea_ax, encode_ea_ax),
        instruction!(MASK_OUT_EA, OP_ADDI | BYTE_SIZED, EA_DATA_ALTERABLE, Size::Byte, "ADDI", valid_ea, decode_imm_ea, is_imm_ea, encode_imm_ea),
        instruction!(MASK_OUT_EA, OP_ADDI | WORD_SIZED, EA_DATA_ALTERABLE, Size::Word, "ADDI", valid_ea, decode_imm_ea, is_imm_ea, encode_imm_ea),
        instruction!(MASK_OUT_EA, OP_ADDI | LONG_SIZED, EA_DATA_ALTERABLE, Size::Long, "ADDI", valid_ea, decode_imm_ea, is_imm_ea, encode_imm_ea),
        instruction!(MASK_OUT_EA, OP_MOVE | WORD_MOVE | MOVE_TO_AN, EA_DATA_ALTERABLE, Size::Word, "MOVEA", valid_ea, decode_ea_ea, is_ea_ea, encode_ea_ea),
        instruction!(MASK_OUT_EA, OP_MOVE | LONG_MOVE | MOVE_TO_AN, EA_DATA_ALTERABLE, Size::Long, "MOVEA", valid_ea, decode_ea_ea, is_ea_ea, encode_ea_ea),
        instruction!(MASK_OUT_EA, OP_MOVE2 | MOVE_TO_SR, EA_DATA, Size::Word, "MOVE", valid_ea, decode_ea_sr, is_ea_sr, encode_just_ea),
        instruction!(MASK_OUT_EA, OP_MOVE2 | MOVE_TO_CCR, EA_DATA, Size::Word, "MOVE", valid_ea, decode_ea_ccr, is_ea_ccr, encode_just_ea),
        instruction!(MASK_OUT_EA_EA, OP_MOVE | BYTE_MOVE, EA_DATA_ALTERABLE, Size::Byte, "MOVE", valid_ea_ea, decode_ea_ea, is_ea_ea, encode_ea_ea),
        instruction!(MASK_OUT_EA_EA, OP_MOVE | WORD_MOVE, EA_DATA_ALTERABLE, Size::Word, "MOVE", valid_ea_ea, decode_ea_ea, is_ea_ea, encode_ea_ea),
        instruction!(MASK_OUT_EA_EA, OP_MOVE | LONG_MOVE, EA_DATA_ALTERABLE, Size::Long, "MOVE", valid_ea_ea, decode_ea_ea, is_ea_ea, encode_ea_ea),
    ]
}

#[cfg(test)]
mod tests {
    use memory::{MemoryVec, Memory};
    use assembler::{Assembler, encode_instruction};
    use disassembler::{disassemble, disassemble_first};
    use super::Exception;
    use assembler::adjust_size;

    #[test]
    fn roundtrips_from_opcode() {
        let opcode = 0xd511;
        let mut mem = &mut MemoryVec::new16(0, vec![opcode]);
        let asm = {
            let inst = disassemble_first(mem);
            format!(" {}", inst)
        };
        let pc = 0;
        let a = Assembler::new();
        let inst = a.parse_assembler(asm.as_str());
        let new_pc = encode_instruction(asm.as_str(), &inst, pc, mem);
        assert_eq!(2, new_pc);
        assert_eq!(opcode, mem.read_word(pc));
    }
    #[test]
    fn roundtrips_from_asm() {
        let mut mem = &mut MemoryVec::new();
        let pc = 0;
        let asm = " ADD.B\tD2,(A1)";
        let a = Assembler::new();
        let inst = a.parse_assembler(asm);
        encode_instruction(asm, &inst, pc, mem);
        let inst = disassemble_first(mem);

        assert_eq!(asm, format!(" {}", inst));
    }

    #[test]
    // #[ignore]
    fn roundtrips() {
        let a = Assembler::new();
        for opcode in 0x0000..0xffff {
            let pc = 0;
            let extension_word_mask = 0b1111_1000_1111_1111; 
            // bits 8-10 should always be zero in the ea extension word
            // as we don't know which word will be seen as the ea extension word
            // (as opposed to immediate operand values) just make sure these aren't set.
            let dasm_mem = &mut MemoryVec::new16(0, vec![opcode, 0x001f, 0x00a4, 0x1234 & extension_word_mask, 0x5678 & extension_word_mask]);
            // println!("PREDASM {:04x}", opcode);
            match disassemble(pc, dasm_mem) {
                Err(Exception::IllegalInstruction(opcode, _)) => (), //println!("{:04x}:\t\tinvalid", opcode),
                Ok(inst_text) => {
                    let asm = format!("\t{}", inst_text);
                    let unsized_inst = a.parse_assembler(asm.as_str());
                    let inst = adjust_size(&unsized_inst);
                    let mut asm_mem = &mut MemoryVec::new();
                    let new_pc = encode_instruction(asm.as_str(), &inst, pc, asm_mem);
                    assert_eq!(inst.length()*2, new_pc);
                    let new_opcode = asm_mem.read_word(pc);
                    if opcode != new_opcode {
                        panic!("{:04x} | {:04x}: {}", opcode, new_opcode, asm);
                    } else {
                        println!("{:04x}: disassembled as {}, parsed as {}, assembled to {:04x}", opcode, asm, inst, new_opcode);
                    }
                    if inst.length() > 1 {
                        let old_ex1 = dasm_mem.read_word(pc+2);
                        let new_ex1 = asm_mem.read_word(pc+2);
                        if old_ex1 != new_ex1 {println!("mismatching extension word: ew1: {:08x} {:08x}", old_ex1, new_ex1)};
                        assert_eq!(old_ex1, new_ex1);
                    };
                    if inst.length() > 2 {
                        let old_ex2 = dasm_mem.read_word(pc+4);
                        let new_ex2 = asm_mem.read_word(pc+4);
                        if old_ex2 != new_ex2 {println!("mismatching extension word: ew2: {:08x} {:08x}", old_ex2, new_ex2)};
                        assert_eq!(old_ex2, new_ex2);
                    };
                    if inst.length() > 3 {
                        let old_ex3 = dasm_mem.read_word(pc+6);
                        let new_ex3 = asm_mem.read_word(pc+6);
                        if old_ex3 != new_ex3 {println!("mismatching extension word: ew3: {:08x} {:08x}", old_ex3, new_ex3)};
                        assert_eq!(old_ex3, new_ex3);
                    };
                }
            }
        }
    }
}