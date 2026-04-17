use common::{emulator, memory, tape::Tape};
use reactive_stores::Store;
use ux::u6;

use crate::components::terminal::Terminal;

pub fn load_tape() -> Tape {
    let data = include_str!("../../../examples/PI.76");

    data.lines()
        .filter_map(|value| {
            if value.is_empty() {
                None
            } else {
                let value = value.trim().parse::<u32>().unwrap();
                let value = u6::try_from(value).unwrap();
                Some(value)
            }
        })
        .collect::<Vec<u6>>()
}

#[derive(Debug, Store, Clone)]
pub struct TapeFile {
    pub name: String,
    pub length: usize,
}

#[derive(Debug, Store)]
pub struct MachineState {
    pub emulator: emulator::State,
    pub terminal: Terminal,
    pub tape_file: Option<TapeFile>,
    pub show_memory: bool,
}

impl MachineState {
    pub fn new() -> Self {
        let memory = memory::load();
        let tape = load_tape();

        MachineState {
            emulator: emulator::State::new(memory, tape),
            terminal: Terminal::new(),
            tape_file: None,
            show_memory: false,
        }
    }

    pub fn step(&mut self, verbose: bool) {
        let write_to_terminal = |str: &str| self.terminal.push_string(str);

        self.emulator.step(verbose, write_to_terminal)
    }

    pub fn input_async(&mut self, key_code: u8) {
        let write_to_terminal = |str: &str| self.terminal.push_string(str);

        self.emulator
            .io
            .input_async(&mut self.emulator.memory, key_code, write_to_terminal)
    }
}
