use std::convert::From;
use ux::u18;

use crate::logger;
use crate::tape::Tape;

#[cfg(feature = "reactive_stores")]
use reactive_stores::Store;

#[derive(Debug)]
pub enum Mode {
    TermOutput,
    TermInput,
    TapeOutput,
    TapeInput,
}

impl TryFrom<u18> for Mode {
    type Error = String;

    fn try_from(value: u18) -> Result<Self, Self::Error> {
        match u32::from(value) {
            9 => Ok(Mode::TermOutput),
            11 => Ok(Mode::TermOutput),
            25 => Ok(Mode::TermInput),
            105 => Ok(Mode::TapeInput),
            _ => Err(format!("Unknown External Function {}", value)),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "reactive_stores", derive(Store))]
pub struct IO {
    pub tacw: u18,
    pub iacw: u18,

    pub state: Mode,
    pub tape: Tape,
}

impl IO {
    pub fn new(tape: Tape) -> Self {
        Self {
            tacw: u18::from(0u8),
            iacw: u18::from(0u8),
            state: Mode::TermOutput,
            tape,
        }
    }

    pub fn external_function(&mut self, memory: &Vec<u18>, channel: u8, verbose: bool) {
        let external_function = memory[u32::from(self.iacw) as usize];
        let external_function = Mode::try_from(external_function).unwrap();

        if verbose {
            logger!(
                "External Function {:?} on channel {}",
                external_function,
                channel
            );
        }

        self.state = external_function;
    }

    // Convert the input to the character set used by the Teletype
    fn to_tty_char(word: u18) -> Option<char> {
        match u8::try_from(word & u18::new(0b0111_1111)).unwrap() {
            // The only two control characters the terminal appears to support
            10 => Some('\n'),
            13 => Some('\r'),
            0..=31 => None,
            // This range is just standard ASCII
            c @ 32..=93 => char::from_u32(c as u32),
            // These characters appear to render as non-standard ASCII
            94 => Some('↑'),
            95 => Some('←'),
            // The lowercase charset is mapped back onto the uppercase charset
            c @ 96..=123 => char::from_u32(c as u32 - 32),
            124..=255 => None,
        }
    }

    pub fn write<F>(&self, memory: &Vec<u18>, channel: u8, verbose: bool, mut write_to_terminal: F)
    where
        F: FnMut(&str),
    {
        let low_idx: usize = u32::from(self.iacw) as usize;
        let high_dx: usize = u32::from(self.tacw) as usize;

        let substring: String = memory[low_idx..=high_dx]
            .iter()
            .filter_map(|memory_item| Self::to_tty_char(*memory_item))
            .collect();

        if verbose {
            logger!("OUTPUT[{}]: {}", channel, substring);
        }

        write_to_terminal(&substring);
    }

    pub fn read(&mut self, memory: &mut Vec<u18>, channel: u8, verbose: bool) {
        if verbose {
            logger!("INPUT[{}]: ", channel);
        }

        match self.state {
            Mode::TapeInput => {
                let in_char = u18::from(self.tape.remove(0));

                memory[u32::from(self.iacw) as usize] = in_char;

                self.iacw = self.iacw + u18::from(1u8);
            }
            Mode::TermInput | _ => (),
        };
    }

    fn of_tty_char(value: u8) -> u18 {
        match value {
            // The lowercase character set is mapped to the uppercase characters
            c @ 97..=122 => u18::from(c - 32),
            value => u18::from(value),
        }
    }

    pub fn input_async<F>(&mut self, memory: &mut Vec<u18>, value: u8, mut write_to_terminal: F)
    where
        F: FnMut(&str),
    {
        match self.state {
            Mode::TermInput => {
                let value = Self::of_tty_char(value);

                // Stop inputting when we exceed the tacw (unclear if this is hardware-accurate,
                // but it makes \r\n input work)
                if self.iacw <= self.tacw {
                    memory[u32::from(self.iacw) as usize] = value;
                    self.iacw = self.iacw + u18::from(1u8);
                }

                if let Some(char) = Self::to_tty_char(value) {
                    let echo = char.to_string();
                    write_to_terminal(&echo);
                }
            }
            _ => (),
        }
    }
}
