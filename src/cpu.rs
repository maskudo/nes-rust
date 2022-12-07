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
            stack_ptr: 0,
            memory: [0; 0xFFFF],
        }
    }

    fn set_flag(&mut self, flag: Flags) {
        self.status.insert(flag);
    }

    fn clear_flag(&mut self, flag: Flags) {
        self.status.remove(flag);
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let low = self.mem_read(pos) as u16;
        let high = self.mem_read(pos + 1) as u16;
        (high << 8) | (low as u16)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let high = (data >> 8) as u8;
        let low = (data & 0xff) as u8;
        self.mem_write(pos, low);
        self.mem_write(pos + 1, high);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_ptr = 0xFD;
        //INTERRUPT_DISABLE and UNUSED set to true
        self.status = Flags::from_bits_truncate(0b100100);

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
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

    fn cpy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let result = self.register_y - value;
        self.update_carry_flag(result);
        self.update_zero_and_negative_flags(result);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    pub fn run(&mut self) {
        loop {
            let opcode = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            match opcode {
                // todo adc

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

                // CPY
                0xc0 | 0xc4 | 0xCC => {
                    self.cpy(&OPCODE_MAP[&opcode].mode);
                }

                // INX
                0xe8 => {
                    if self.register_x == 0xff {
                        self.register_x = 0x00;
                    } else {
                        self.register_x += 1;
                    }
                    self.update_zero_and_negative_flags(self.register_x);
                }

                //LDA
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&OPCODE_MAP[&opcode].mode);
                }

                // TAX
                0xAA => {
                    self.register_x = self.register_a;
                    self.update_zero_and_negative_flags(self.register_x);
                }

                _ => todo!(),
            }
            if program_counter_state == self.program_counter {
                self.program_counter += (OPCODE_MAP[&opcode].length - 1) as u16;
            }
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
