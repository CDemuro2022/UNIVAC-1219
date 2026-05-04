use std::fmt;

use crate::arith;
use crate::instruction::Instruction;
use crate::io::{IO, Mode};
use crate::logger;
use crate::tape::Tape;
use ux::{u3, u5, u6, u12, u18, u36};

mod address {
    pub const _RTC_MONITOR_INTERRUPT: usize = 0o12;
    pub const _RTC_OVERFLOW_INTERRUPT: usize = 0o13;
    pub const _RTC_GOAL: usize = 0o14;
    pub const RTC: usize = 0o15;
}

#[cfg(feature = "reactive_stores")]
use reactive_stores::Store;

#[cfg_attr(feature = "reactive_stores", derive(Store))]
pub struct State {
    pub time_microseconds: u64,
    pub running: bool,

    pub icr: u3,
    pub sr: u5,

    pub au: u18,
    pub al: u18,
    pub p: u18,

    pub compare: bool,
    pub equal: bool,
    pub greater: bool,
    pub carry: bool,

    pub stop: u6,
    pub stopped: u6,
    pub skip: u5,

    pub rtc_enabled: bool,
    pub real_time_clock: u64,

    pub io: IO,

    pub memory: Vec<u18>,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "P: {:0>6o} AU: {:0>6o} AL: {:0>6o} ICR: {:0>1o} SR: {:0>5b}",
            self.p, self.au, self.al, self.icr, self.sr
        )
    }
}

impl State {
    pub fn new(memory: Vec<u18>, tape: Tape) -> Self {
        Self {
            time_microseconds: 0u64,
            running: false,
            icr: u3::default(),
            sr: u5::default(),
            au: u18::default(),
            al: u18::default(),
            p: u18::default(),
            compare: false,
            equal: false,
            greater: false,
            carry: false,
            stop: u6::default(),
            stopped: u6::default(),
            skip: u5::default(),
            rtc_enabled: false,
            real_time_clock: 0u64,
            memory,
            io: IO::new(tape),
        }
    }

    fn b_addr(&self) -> usize {
        let icr = usize::from(u16::from(self.icr));
        if icr == 0 { 8 } else { icr }
    }

    fn get_b(&self) -> u18 {
        self.memory[self.b_addr()]
    }

    fn set_b(&mut self, b: u18) {
        let addr = self.b_addr();
        self.memory[addr] = b;
    }

    fn get_au_al(&self) -> u36 {
        u36::from(self.au) << 18 | u36::from(self.al)
    }

    fn set_au_al(&mut self, a: u36) {
        self.al = u18::try_from(a & u36::from(0b111_111_111_111_111_111u32)).unwrap();
        self.au = u18::try_from((a >> 18) & u36::from(0b111_111_111_111_111_111u32)).unwrap();
    }

    fn read(&self, addr: u18) -> u18 {
        let addr: usize = u18::try_into(addr).unwrap();

        return self.memory[addr];
    }

    fn read_36(&self, addr: u18) -> u36 {
        let addr_val: u32 = addr.into();
        if addr_val % 2 != 0 {
            panic!("36-bit operation requires even address, got {:#o}", addr);
        }
        let lower = self.read(addr);
        let upper = self.read(addr + 1u8.into());
        u36::from(upper) << 18 | u36::from(lower)
    }

    fn write(&mut self, addr: u18, data: u18) {
        let addr: usize = u18::try_into(addr).unwrap();

        self.memory[addr] = data;
    }

    fn p_addr(&self, addr: u12) -> u18 {
        (self.p & u18::new(0o770000)) | u18::from(addr)
    }

    fn local_addr(&self, addr: u12) -> u18 {
        let sr_active = (u8::from(self.sr)) & 0b1000u8 != 0u8;

        if sr_active {
            let upper_bits = u8::from(self.sr) & 0b111u8 | ((u8::from(self.sr) & 0b10000u8) >> 1);

            (u18::from(upper_bits) << 12) | u18::from(addr)
        } else {
            let upper_mask: u18 = u18::try_from(0b111_111_000_000_000_000u32).unwrap();

            (self.p & upper_mask) | u18::from(addr)
        }
    }

    pub fn step<F>(&mut self, verbose: bool, write_to_terminal: F) -> ()
    where
        F: FnMut(&str),
    {
        let code = self.read(self.p);
        let instruction = Instruction::parse(u18::try_into(code).unwrap()).unwrap();
        let instruction_time = instruction.microseconds();

        self.stopped = u6::new(0);

        self.time_microseconds += instruction_time;

        if self.rtc_enabled {
            self.real_time_clock += instruction_time;

            // Convert to 1024Hz clock ticks from microseconds
            let rtc_register = u18::new(((self.real_time_clock * 1024) / 1_000_000) as u32);

            self.memory[address::RTC] = rtc_register;
        }

        let compare = self.compare;
        let equal = self.equal;
        let greater = self.greater;

        self.compare = false;
        self.equal = false;
        self.greater = false;

        if verbose {
            logger!(
                "{:0>10}µs\t{:0>6o}: {:?}\t{:?}",
                self.time_microseconds,
                self.p,
                instruction,
                self
            );
        }

        match instruction {
            Instruction::CMAL { y } => {
                let val = self.read(self.local_addr(y));

                // Convert both values to signed for proper comparison
                let al_signed = arith::ones_to_signed_18(self.al);
                let val_signed = arith::ones_to_signed_18(val);

                self.compare = true;
                self.equal = al_signed == val_signed;
                self.greater = al_signed >= val_signed;
            }
            Instruction::SLSU { y } => {
                let mask = self.au;
                let mem = self.read(self.local_addr(y));
                self.al = (self.al & !mask) | (mem & mask);
            }
            Instruction::SLSUB { y } => {
                let mask = self.au;
                let mem = self.read(self.local_addr(y) + self.get_b());
                self.al = (self.al & !mask) | (mem & mask);
            }
            Instruction::ENTAU { y } => {
                self.au = self.read(self.local_addr(y));
            }
            Instruction::ENTAUB { y } => {
                self.au = self.read(self.local_addr(y) + self.get_b());
            }
            Instruction::ENTAL { y } => {
                self.al = self.read(self.local_addr(y));
            }
            Instruction::ENTALB { y } => {
                self.al = self.read(self.local_addr(y) + self.get_b());
            }
            Instruction::ADDAL { y } => {
                let operand = self.read(self.local_addr(y));
                self.al = arith::add_18(self.al, operand);
            }
            Instruction::ADDALB { y } => {
                let operand = self.read(self.local_addr(y) + self.get_b());
                self.al = arith::add_18(self.al, operand);
            }
            Instruction::SUBAL { y } => {
                let operand = self.read(self.local_addr(y));
                self.al = arith::sub_18(self.al, operand);
            }
            Instruction::SUBALB { y } => {
                let operand = self.read(self.local_addr(y) + self.get_b());
                self.al = arith::sub_18(self.al, operand);
            }
            Instruction::ADDA { y } => {
                let addend = self.read_36(self.local_addr(y));
                let result = arith::add_36(self.get_au_al(), addend);
                self.set_au_al(result);
            }
            Instruction::ADDAB { y } => {
                let addend = self.read_36(self.local_addr(y) + self.get_b());
                let result = arith::add_36(self.get_au_al(), addend);
                self.set_au_al(result);
            }
            Instruction::SUBA { y } => {
                let subtrahend = self.read_36(self.local_addr(y));
                let result = arith::sub_36(self.get_au_al(), subtrahend);
                self.set_au_al(result);
            }
            Instruction::SUBAB { y } => {
                let subtrahend = self.read_36(self.local_addr(y) + self.get_b());
                let result = arith::sub_36(self.get_au_al(), subtrahend);
                self.set_au_al(result);
            }
            Instruction::MULAL { y } => {
                let operand = self.read(self.local_addr(y));
                let result = arith::mul_18(self.al, operand);
                self.set_au_al(result);
            }
            Instruction::MULALB { y } => {
                let operand = self.read(self.local_addr(y) + self.get_b());
                let result = arith::mul_18(self.al, operand);
                self.set_au_al(result);
            }
            Instruction::DIVA { y } => {
                let divisor = self.read(self.local_addr(y));
                let (quotient, remainder) = arith::div(self.get_au_al(), divisor);
                self.al = quotient;
                self.au = remainder;
            }
            Instruction::DIVAB { y } => {
                let divisor = self.read(self.local_addr(y) + self.get_b());
                let (quotient, remainder) = arith::div(self.get_au_al(), divisor);
                self.al = quotient;
                self.au = remainder;
            }
            Instruction::IRJP { y } => {
                let target = self.read(self.p_addr(y));

                self.memory[u32::from(target) as usize] = self.p + (1u16).into();
                self.p = target + (1u16).into();
                return;
            }
            Instruction::IRJPB { y } => {
                let target = self.read(self.p_addr(y) + self.get_b());

                self.memory[u32::from(target) as usize] = self.p + (1u16).into();
                self.p = target + (1u16).into();
                return;
            }
            Instruction::ENTB { y } => {
                let addr = self.local_addr(y);
                let b = self.read(addr);

                self.set_b(b);
            }
            Instruction::ENTBB { y } => {
                self.set_b(self.read(self.local_addr(y) + self.get_b()));
            }
            Instruction::JP { y } => {
                self.p = self.p_addr(y);
                return;
            }
            Instruction::JPB { y } => {
                self.p = self.p_addr(y) + self.get_b();
                return;
            }
            Instruction::ENTBK { u } => {
                self.set_b(arith::sign_extend_12_to_18(u));
            }
            Instruction::ENTBKB { u } => {
                self.set_b(arith::add_18(self.get_b(), arith::sign_extend_12_to_18(u)));
            }
            Instruction::CL { y } => {
                self.write(self.local_addr(y), 0u8.into());
            }
            Instruction::CLB { y } => {
                self.write(self.local_addr(y) + self.get_b(), 0u8.into());
            }
            Instruction::STRB { y } => {
                self.write(self.local_addr(y), self.get_b());
            }
            Instruction::STRBB { y } => {
                self.write(self.local_addr(y) + self.get_b(), self.get_b());
            }
            Instruction::STRAL { y } => {
                self.write(self.local_addr(y), self.al);
            }
            Instruction::STRALB { y } => {
                self.write(self.local_addr(y) + self.get_b(), self.al);
            }
            Instruction::STRAU { y } => {
                self.write(self.local_addr(y), self.au);
            }
            Instruction::STRAUB { y } => {
                self.write(self.local_addr(y) + self.get_b(), self.au);
            }
            Instruction::SLSET { y } => {
                self.al = self.al | self.read(self.p_addr(y));
            }
            Instruction::SLCL { y } => {
                self.al = self.al & self.read(self.p_addr(y));
            }
            Instruction::SLCP { y } => {
                self.al = self.al ^ self.read(self.p_addr(y));
            }
            Instruction::IJP { y } => {
                self.p = self.read(self.p_addr(y));
                return;
            }
            Instruction::BSK { y } => {
                let addr = self.p_addr(y);
                let target = self.read(addr);

                let b = self.get_b();

                if b == target {
                    self.p = self.p + 1u8.into();
                } else {
                    self.set_b(arith::add_18(b, u18::new(1)));
                }
            }
            Instruction::ISK { y } => {
                let addr = self.p_addr(y);

                let idx = self.read(addr);

                if idx == 0u8.into() {
                    self.p = self.p + 1u8.into();
                } else {
                    self.write(addr, idx - 1u8.into());
                }
            }
            Instruction::JPAUZ { y } => {
                let do_jump = if compare {
                    equal
                } else {
                    arith::is_zero(self.au)
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::JPALZ { y } => {
                let do_jump = if compare {
                    equal
                } else {
                    arith::is_zero(self.al)
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::JPAUNZ { y } => {
                let do_jump = if compare {
                    !equal
                } else {
                    !arith::is_zero(self.au)
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::JPALNZ { y } => {
                let do_jump = if compare {
                    !equal
                } else {
                    !arith::is_zero(self.al)
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::JPAUP { y } => {
                let do_jump = if compare {
                    greater
                } else {
                    u32::from(self.au) & 0b100000_000000_000000 == 0
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::JPALP { y } => {
                let do_jump = if compare {
                    greater
                } else {
                    u32::from(self.al) & 0b100000_000000_000000 == 0
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::JPAUNG { y } => {
                let do_jump = if compare {
                    !greater
                } else {
                    u32::from(self.au) & 0b100000_000000_000000 != 0
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::JPALNG { y } => {
                let do_jump = if compare {
                    !greater
                } else {
                    u32::from(self.al) & 0b100000_000000_000000 != 0
                };

                if do_jump {
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::ENTALK { u } => {
                self.al = arith::sign_extend_12_to_18(u);
            }
            Instruction::ADDALK { u } => {
                let operand = arith::sign_extend_12_to_18(u);
                self.al = arith::add_18(self.al, operand);
            }
            Instruction::STRICR { y } => {
                let addr = self.p_addr(y);
                let upper_bits = self.read(addr) & u18::new(0o777700);
                self.write(addr, upper_bits | u18::from(self.icr));
            }
            Instruction::BJP { y } => {
                let b_val = self.get_b();
                if b_val != u18::new(0) {
                    self.set_b(b_val - u18::new(1));
                    self.p = self.p_addr(y);
                    return;
                }
            }
            Instruction::STRADR { y } => {
                let addr = self.p_addr(y);
                let upper_bits = self.read(addr) & u18::new(0o770000);
                let al_lower = self.al & u18::new(0o007777);
                self.write(addr, upper_bits | al_lower);
            }
            Instruction::STRSR { y } => {
                let addr = self.p_addr(y);
                let upper_bits = self.read(addr) & u18::new(0o777700);
                self.write(addr, upper_bits | u18::from(self.sr));
                self.sr = self.sr & u5::new(0b10111);
            }
            Instruction::RJP { y } => {
                let target = self.p_addr(y);

                self.memory[u32::from(target) as usize] = self.p + (1u16).into();
                self.p = target + (1u16).into();
                return;
            }
            Instruction::IN { k } => {
                self.io.tacw = self.read(self.p + (1u16).into());
                self.io.iacw = self.read(self.p + (2u16).into());

                self.io.read(&mut self.memory, k.into(), verbose);

                self.p = self.p + (2u16).into();
            }
            Instruction::OUT { k } => {
                self.io.tacw = self.read(self.p + (1u16).into());
                self.io.iacw = self.read(self.p + (2u16).into());

                self.io
                    .write(&self.memory, k.into(), verbose, write_to_terminal);

                self.p = self.p + (2u16).into();
            }
            Instruction::EXF { k } => {
                self.io.tacw = self.read(self.p + (1u16).into());
                self.io.iacw = self.read(self.p + (2u16).into());

                self.io.external_function(&self.memory, k.into(), verbose);

                self.p = self.p + (2u16).into();
            }
            Instruction::RTC => {
                self.rtc_enabled = true;
            }
            Instruction::SKPIIN { k: _ } => match self.io.state {
                Mode::TermInput => {
                    if self.io.iacw != self.io.tacw {
                        self.p = self.p + (1u16).into();
                    }
                }
                _ => {
                    self.p = self.p + (1u16).into();
                }
            },
            Instruction::SKPOIN { k: _ } => {
                self.p = self.p + (1u16).into();
            }
            Instruction::SKPFIN { k: _ } => {
                self.p = self.p + (1u16).into();
            }
            Instruction::RSHA { k } => {
                let a = self.get_au_al();
                let result = arith::rshift_36(a, k);
                self.set_au_al(result);
            }
            Instruction::RSHAU { k } => {
                self.au = arith::rshift_18(self.au, k);
            }
            Instruction::RSHAL { k } => {
                self.al = arith::rshift_18(self.al, k);
            }
            Instruction::LSHAU { k } => {
                for _ in 0..u32::from(k) {
                    let top_bit = (self.au >> 17) & u18::new(1);
                    self.au = (self.au << 1) | top_bit;
                }
            }
            Instruction::LSHAL { k } => {
                let mut a = self.al;

                for _ in 0..u32::from(k) {
                    let top_bit = a & u18::try_from(0b100000_000000_000000u32).unwrap();
                    a = a << 1;

                    a |= u18::from(if top_bit != 0u8.into() { 1u8 } else { 0u8 });
                }

                self.al = a;
            }
            Instruction::LSHA { k } => {
                let mut a = self.get_au_al();

                for _ in 0..u32::from(k) {
                    let top_bit =
                        a & u36::try_from(0b100000_000000_000000_000000_000000_000000u64).unwrap();
                    a = a << 1;

                    a |= u36::from(if top_bit != 0u8.into() { 1u8 } else { 0u8 });
                }

                self.set_au_al(a);
            }
            Instruction::RND => {
                let msb = self.al >> 17 & u18::new(1);
                if self.au <= u18::new(0o377777) {
                    self.al = arith::add_18(self.au, msb);
                } else {
                    self.al = arith::sub_18(self.au, u18::new(1) - msb);
                }
            }
            Instruction::CPAL => {
                // Per docs: positive zero (all zeros) remains all zeros
                if !arith::is_zero(self.al) {
                    self.al = arith::complement_18(self.al);
                }
            }
            Instruction::CPAU => {
                // Per docs: positive zero (all zeros) remains all zeros
                if !arith::is_zero(self.au) {
                    self.au = arith::complement_18(self.au);
                }
            }
            Instruction::CPA => {
                // Per docs: positive zero (all zeros) remains all zeros
                // For 36-bit A register, check if entire A is zero
                if !(arith::is_zero(self.au) && arith::is_zero(self.al)) {
                    self.au = arith::complement_18(self.au);
                    self.al = arith::complement_18(self.al);
                }
            }
            Instruction::ENTICR { k } => {
                self.icr = u3::try_from(k).unwrap();
            }
            Instruction::ENTSR { k } => {
                self.sr = u5::try_from(k).unwrap();
            }
            Instruction::STOP { k } => {
                if (k & self.stop != u6::new(0))  || (k >= u6::new(0b100000)) {
                    self.stopped = k & self.stop;
                    self.running = false;
                    logger!("\nSTOPPED: {:06b}", self.stopped);
                }
            }
            _ => {
                logger!("Unknown instruction! {:?} at {:0>6o}", instruction, self.p);
            }
        }

        self.p = self.p + (1u16).into();
    }
}
