//the cpu is a mos technology 6502 microprocessor

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    pub stack_ptr: u16,
    pub memory: [u8; 0xFFFF],
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            stack_ptr: 0,
            memory: [0; 0xFFFF],
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.run();
    }

    fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.program_counter = 0x8000;
    }
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // changing zero flag
        if result == 0 {
            self.status = self.status | 0b0000_0010;
        } else {
            self.status = self.status & 0b0111_1111;
        }

        // changing negative flag
        if result & 0b1000_0000 != 0 {
            self.status |= 0b1000_0000;
        } else {
            self.status &= 0b0111_1111;
        }
    }
    fn update_carry_flag(&mut self, result: u8) {
        if result > 0 {
            self.status |= 0b0000_0001;
        } else {
            self.status &= 0b1111_1110;
        }
    }

    pub fn run(&mut self) {
        loop {
            let opcode = self.mem_read(self.program_counter);
            self.program_counter += 1;

            match opcode {
                // LDA
                0xA9 => {
                    let param = self.mem_read(self.program_counter);
                    self.program_counter += 1;
                    self.register_a = param;

                    self.update_zero_and_negative_flags(self.register_a);
                }

                // BRK
                0x00 => {
                    return;
                }

                // TAX
                0xAA => {
                    self.register_x = self.register_a;
                    self.update_zero_and_negative_flags(self.register_x);
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

                // CPY
                0xc0 => {
                    let param = self.mem_read(self.program_counter);
                    self.program_counter += 1;

                    let result = self.register_y - param;
                    self.update_carry_flag(result);
                    self.update_zero_and_negative_flags(result);
                }

                _ => todo!(),
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
        assert!(cpu.status & 0b0000_0010 == 0b00);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.register_a = 10;
        cpu.load_and_run(vec![0xaa, 0x00]);

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
        cpu.register_x = 0xff;
        cpu.load_and_run(vec![0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1);
    }
}
