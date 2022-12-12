//the cpu is a mos technology 6502 microprocessor

use crate::opcodes::OPCODE_MAP;
use bitflags::bitflags;

bitflags! {
    pub struct Flags: u8{
        const CARRY = (1<<0);
        const ZERO = (1<<1);
        const INTERRUPT_DISABLE = (1<<2);
        const DECIMAL_MODE = (1<<3);
        const BREAK = (1<<4);
        const UNUSED = (1<<5);
        const OVERFLOW = (1<<6);
        const NEGATIVE = (1<<7);
    }
}

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: Flags,
    pub program_counter: u16,
    pub stack_ptr: u8,
    pub memory: [u8; 0xFFFF],
}

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: Flags::from_bits_truncate(0b100100),
            program_counter: 0,
            stack_ptr: STACK_RESET,
            memory: [0; 0xFFFF],
        }
    }

    fn set_flag(&mut self, flag: Flags) {
        self.status.insert(flag);
    }

    fn clear_flag(&mut self, flag: Flags) {
        self.status.remove(flag);
    }

    pub fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn mem_read_u16(&self, pos: u16) -> u16 {
        let low = self.mem_read(pos) as u16;
        let high = self.mem_read(pos + 1) as u16;
        (high << 8) | (low as u16)
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    pub fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let high = (data >> 8) as u8;
        let low = (data & 0xff) as u8;
        self.mem_write(pos, low);
        self.mem_write(pos + 1, high);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_ptr = STACK_RESET;
        //INTERRUPT_DISABLE and UNUSED set to true
        self.status = Flags::from_bits_truncate(0b100100);

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x0600..(0x0600 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x0600);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // changing zero flag
        if result == 0 {
            self.set_flag(Flags::ZERO);
        } else {
            self.clear_flag(Flags::ZERO);
        }

        // changing negative flag
        if result & 0b1000_0000 != 0 {
            self.set_flag(Flags::NEGATIVE);
        } else {
            self.clear_flag(Flags::NEGATIVE);
        }
    }

    fn update_carry_flag(&mut self, result: u8) {
        if result > 0 {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
    }

    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK as u16) + self.stack_ptr as u16, data);
        self.stack_ptr = self.stack_ptr.wrapping_sub(1);
    }

    fn stack_push_u16(&mut self, data: u16) {
        let high = (data >> 8) as u8;
        let low = (data & 0xff) as u8;
        self.stack_push(high);
        self.stack_push(low);
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_ptr = self.stack_ptr.wrapping_add(1);
        self.mem_read((STACK as u16) + self.stack_ptr as u16)
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let low = self.stack_pop() as u16;
        let high = self.stack_pop() as u16;

        high << 8 | low
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,

            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,

            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),

            AddressingMode::ZeroPage_X => {
                let iaddr = self.mem_read(self.program_counter);
                iaddr.wrapping_add(self.register_x) as u16
            }

            AddressingMode::ZeroPage_Y => {
                let iaddr = self.mem_read(self.program_counter);
                iaddr.wrapping_add(self.register_y) as u16
            }

            AddressingMode::Absolute_X => self
                .mem_read_u16(self.program_counter)
                .wrapping_add(self.register_x as u16),

            AddressingMode::Absolute_Y => self
                .mem_read_u16(self.program_counter)
                .wrapping_add(self.register_y as u16),

            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }

            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let sum = self.register_a as u16
            + value as u16
            + (if self.status.contains(Flags::CARRY) {
                1
            } else {
                0
            }) as u16;
        let carry = sum > 0xff;
        if carry {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        let result = sum as u8;
        if (value ^ result) & (result ^ self.register_a) & 0x80 != 0 {
            self.set_flag(Flags::OVERFLOW);
        } else {
            self.clear_flag(Flags::OVERFLOW);
        }
        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let result = self.register_a & value;
        self.register_a = result;
        self.update_zero_and_negative_flags(result);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        if let AddressingMode::NoneAddressing = mode {
            let data = self.register_a;
            self.update_carry_flag(data);
            self.register_a = data << 1;
        } else {
            let addr = self.get_operand_address(mode);
            let value = self.mem_read(addr);

            self.update_carry_flag(value);
            let result = value << 1;
            self.mem_write(addr, result);
            self.update_zero_and_negative_flags(result);
        }
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let jump = self.mem_read(self.program_counter) as i8;
            let addr = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(jump as u16);

            self.program_counter = addr;
        }
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let result = self.register_a & value;
        self.update_zero_and_negative_flags(result);
        if result & (1 << 6) == 1 {
            self.set_flag(Flags::OVERFLOW);
        } else {
            self.clear_flag(Flags::OVERFLOW);
        }
    }

    fn compare(&mut self, mode: &AddressingMode, compare_with_reg: u8) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let result = compare_with_reg.wrapping_sub(value);
        self.update_carry_flag(result);
        self.update_zero_and_negative_flags(result);
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let result = value.wrapping_sub(1);
        self.mem_write(addr, result);
        self.update_zero_and_negative_flags(result);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a ^= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let result = value.wrapping_add(1);
        self.mem_write(addr, result);
        self.update_zero_and_negative_flags(result);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn lsr_acc(&mut self) {
        let data = self.register_a;
        if data & 1 == 1 {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        self.register_a = data >> 1;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);

        if data & 1 == 1 {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        data = data >> 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = self.register_a | value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn php(&mut self) {
        let mut flags = self.status.clone();
        flags.insert(Flags::BREAK);
        flags.insert(Flags::UNUSED);
        self.stack_push(flags.bits());
    }

    fn rol_acc(&mut self) {
        let mut value = self.register_a;
        let old_carry = self.status.contains(Flags::CARRY);
        if value >> 7 == 1 {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        value = value << 1;
        if old_carry {
            value = value | 1;
        }
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn rol(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);
        let old_carry = self.status.contains(Flags::CARRY);
        if value >> 7 == 1 {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        value = value << 1;
        if old_carry {
            value = value | 1;
        }
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn ror_acc(&mut self) {
        let mut value = self.register_a;
        let old_carry = self.status.contains(Flags::CARRY);
        if value & 1 == 1 {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        value = value >> 1;
        if old_carry {
            value = value | (1 << 7);
        }
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn ror(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);
        let old_carry = self.status.contains(Flags::CARRY);
        if value & 1 == 1 {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        value = value >> 1;
        if old_carry {
            value = value | (1 << 7);
        }
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);
        value = (value as i8).wrapping_neg().wrapping_sub(1) as u8;

        let sum = self.register_a as u16
            + value as u16
            + (if self.status.contains(Flags::CARRY) {
                1
            } else {
                0
            }) as u16;
        let carry = sum > 0xff;
        if carry {
            self.set_flag(Flags::CARRY);
        } else {
            self.clear_flag(Flags::CARRY);
        }
        let result = sum as u8;
        if (value ^ result) & (result ^ self.register_a) & 0x80 != 0 {
            self.set_flag(Flags::OVERFLOW);
        } else {
            self.clear_flag(Flags::OVERFLOW);
        }
        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        loop {
            let opcode = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            match opcode {
                // ADC
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                    self.adc(&OPCODE_MAP[&opcode].mode)
                }

                //AND
                0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => {
                    self.and(&OPCODE_MAP[&opcode].mode);
                }

                //ASL
                0x0a | 0x06 | 0x16 | 0x1e | 0x0e => self.asl(&OPCODE_MAP[&opcode].mode),

                //BCC
                0x90 => self.branch(!self.status.contains(Flags::CARRY)),

                //BCS
                0xb0 => self.branch(self.status.contains(Flags::CARRY)),

                //BPL
                0x10 => self.branch(!self.status.contains(Flags::NEGATIVE)),

                //BMI
                0x30 => self.branch(self.status.contains(Flags::NEGATIVE)),

                //BVC
                0x50 => self.branch(!self.status.contains(Flags::OVERFLOW)),

                //BVS
                0x70 => self.branch(self.status.contains(Flags::OVERFLOW)),

                //BNE
                0xd0 => self.branch(!self.status.contains(Flags::ZERO)),

                //BEQ
                0xf0 => self.branch(self.status.contains(Flags::ZERO)),

                // BRK
                0x00 => {
                    return;
                }

                //BIT
                0x24 | 0x2c => self.bit(&OPCODE_MAP[&opcode].mode),

                //CLC
                0x18 => self.clear_flag(Flags::CARRY),

                //CLD
                0xd8 => self.clear_flag(Flags::DECIMAL_MODE),

                //CLI
                0x58 => self.clear_flag(Flags::INTERRUPT_DISABLE),

                //CLV
                0xb8 => self.clear_flag(Flags::OVERFLOW),

                //CMP
                0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => {
                    self.compare(&OPCODE_MAP[&opcode].mode, self.register_a);
                }

                // CPX
                0xe0 | 0xe4 | 0xec => {
                    self.compare(&OPCODE_MAP[&opcode].mode, self.register_x);
                }

                // CPY
                0xc0 | 0xc4 | 0xcc => {
                    self.compare(&OPCODE_MAP[&opcode].mode, self.register_y);
                }

                //DEC
                0xc6 | 0xd6 | 0xce | 0xde => self.dec(&OPCODE_MAP[&opcode].mode),

                //DEX
                0xca => self.dex(),
                //dey
                0x88 => self.dey(),

                // EOR
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                    self.eor(&OPCODE_MAP[&opcode].mode)
                }

                //INC
                0xe6 | 0xf6 | 0xee | 0xfe => self.inc(&OPCODE_MAP[&opcode].mode),

                // INX
                0xe8 => self.inx(),

                // INY
                0xc8 => self.iny(),

                //JMP Abs
                0x4c => {
                    let addr = self.mem_read_u16(self.program_counter);
                    self.program_counter = addr;
                }
                // JMP Indirect
                0x6c => {
                    // An original 6502 has does not correctly fetch the target address
                    //if the indirect vector falls on a page boundary
                    //(e.g. $xxFF where xx is any value from $00 to $FF).
                    //In this case fetches the LSB from $xxFF as expected but takes the MSB from $xx00.
                    //This is fixed in some later chips like the 65SC02
                    //so for compatibility always ensure the indirect vector is not at the end of the page.

                    let addr = self.mem_read_u16(self.program_counter);
                    let indirect_ref = if addr & 0xff == 0xff {
                        let low = self.mem_read(addr);
                        let high = self.mem_read(addr & 0xff00);
                        (high as u16) << 8 | (low as u16)
                    } else {
                        self.mem_read_u16(addr)
                    };

                    self.program_counter = indirect_ref;
                }

                //JSR
                0x20 => {
                    self.stack_push_u16(self.program_counter + 2 - 1);
                    let target = self.mem_read_u16(self.program_counter);
                    self.program_counter = target;
                }

                //LDA
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&OPCODE_MAP[&opcode].mode);
                }

                //LDX
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => self.ldx(&OPCODE_MAP[&opcode].mode),

                //LDY
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => self.ldy(&OPCODE_MAP[&opcode].mode),

                //LSR
                0x4a => self.lsr_acc(),
                0x46 | 0x56 | 0x4e | 0x5e => self.lsr(&OPCODE_MAP[&opcode].mode),

                //NOP
                0xea => {}

                //ORA
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                    self.ora(&OPCODE_MAP[&opcode].mode)
                }

                //PHA
                0x48 => self.stack_push(self.register_a),
                //PHP
                0x08 => self.php(),
                //PLA
                0x68 => {
                    let value = self.stack_pop();
                    self.register_a = value;
                    self.update_zero_and_negative_flags(value);
                }

                //PLP
                0x28 => {
                    self.status.bits = self.stack_pop();
                    self.status.remove(Flags::BREAK);
                    self.status.insert(Flags::UNUSED);
                }

                //ROL
                0x2a => self.rol_acc(),
                0x26 | 0x36 | 0x2e | 0x3e => self.rol(&OPCODE_MAP[&opcode].mode),

                //ROR
                0x6a => self.ror_acc(),
                0x66 | 0x76 | 0x6e | 0x7e => self.ror(&OPCODE_MAP[&opcode].mode),

                //RTI
                0x40 => {
                    self.status.bits = self.stack_pop();
                    self.status.remove(Flags::BREAK);
                    self.status.insert(Flags::UNUSED);

                    self.program_counter = self.stack_pop_u16();
                }

                //RTS
                0x60 => self.program_counter = self.stack_pop_u16() + 1,

                //SBC
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                    self.sbc(&OPCODE_MAP[&opcode].mode)
                }

                //SEC
                0x38 => self.set_flag(Flags::CARRY),
                //SED
                0xf8 => self.set_flag(Flags::DECIMAL_MODE),
                //SEI
                0x78 => self.set_flag(Flags::INTERRUPT_DISABLE),

                //STA
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    let addr = self.get_operand_address(&OPCODE_MAP[&opcode].mode);
                    self.mem_write(addr, self.register_a);
                }

                //STX
                0x86 | 0x96 | 0x8e => {
                    let addr = self.get_operand_address(&OPCODE_MAP[&opcode].mode);
                    self.mem_write(addr, self.register_x);
                }

                //STY
                0x84 | 0x94 | 0x8c => {
                    let addr = self.get_operand_address(&OPCODE_MAP[&opcode].mode);
                    self.mem_write(addr, self.register_y);
                }

                // TAX
                0xaa => {
                    self.register_x = self.register_a;
                    self.update_zero_and_negative_flags(self.register_x);
                }

                // TAY
                0xa8 => {
                    self.register_y = self.register_a;
                    self.update_zero_and_negative_flags(self.register_y);
                }

                //TSX
                0xba => {
                    self.register_x = self.stack_ptr;
                    self.update_zero_and_negative_flags(self.register_x);
                }

                //TXA
                0x8a => {
                    self.register_a = self.register_x;
                    self.update_zero_and_negative_flags(self.register_a);
                }

                //TXS
                0x9a => {
                    self.stack_ptr = self.register_x;
                }

                //TYA
                0x98 => {
                    self.register_a = self.register_y;
                    self.update_zero_and_negative_flags(self.register_a);
                }
                _ => {
                    println!("Illegal opcde {}", opcode);
                    todo!();
                }
            }
            if program_counter_state == self.program_counter {
                self.program_counter += (OPCODE_MAP[&opcode].length - 1) as u16;
            }
            callback(self);
        }
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use super::*;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status.bits() & 0b0000_0010 == 0b00);
        assert!(cpu.status.bits() & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status.bits() & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0a, 0xaa, 0x00]);

        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1);
    }

    #[test]
    fn text_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1);
    }
}
