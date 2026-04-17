use std::iter::Peekable;
use ux::{u6, u18};

pub type Tape = Vec<u6>;

pub fn new() -> Tape {
    vec![]
}

pub fn split_bioctal(word: &u18) -> Vec<u6> {
    let upper = u6::try_from((*word >> 12) & u18::new(0o77)).unwrap();
    let middle = u6::try_from((*word >> 6) & u18::new(0o77)).unwrap();
    let lower = u6::try_from(*word & u18::new(0o77)).unwrap();

    vec![upper, middle, lower]
}

pub fn join_bioctal(tape: &mut Peekable<impl Iterator<Item = u6>>) -> u18 {
    let b0: u32 = tape.next().unwrap().into();
    let b1: u32 = tape.next().unwrap().into();
    let b2: u32 = tape.next().unwrap().into();

    u18::try_from((b0 << 12) | (b1 << 6) | b2).unwrap()
}

pub fn serialize_bioctal(tape: &Tape) -> String {
    tape.iter()
        .map(|value| format!(" {:} \n", value))
        .collect::<String>()
}

pub fn deserialize_bioctal(data: &str) -> Tape {
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

pub fn serialize_bin(tape: &Tape) -> Vec<u8> {
    tape.iter().map(|value| u8::from(*value)).collect()
}

pub fn deserialize_bin(data: &[u8]) -> Tape {
    data.iter()
        .map(|value| u6::try_from(*value & 0o77).unwrap())
        .collect()
}
