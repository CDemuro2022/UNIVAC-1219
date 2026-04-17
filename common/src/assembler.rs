use crate::instruction::Instruction;
use crate::memory;
use crate::tape;
use std::collections::BTreeMap;
use ux::{i18, u18};

#[derive(Debug, Clone)]
pub enum Operand {
    Number { value: u18 },
    Lok { offset: i18 },
    Symbol { label: String, offset: i18 },
}

// TODO: Replace panics with proper error handling.

impl Operand {
    fn parse_number(string: &str) -> Option<u18> {
        let value = match string.trim().to_uppercase().strip_prefix("&O") {
            Some(string) => u32::from_str_radix(string.trim(), 8),
            None => match string.trim().strip_prefix("&") {
                Some(string) => u32::from_str_radix(string.trim(), 8),
                None => u32::from_str_radix(string.trim(), 10),
            },
        }
        .ok()?;

        Some(u18::try_from(value).unwrap_or_else(|_| {
            panic!("Value {} (0o{:o}) is too large for 18-bit word. Max is 262143 (0o777777). String: '{}'",
                   value, value, string.trim())
        }))
    }

    fn parse(string: &str) -> Operand {
        match Self::parse_number(string) {
            Some(value) => Operand::Number { value },
            None => {
                let (label, offset) = match (string.split_once("+"), string.split_once("-")) {
                    (None, None) => (string, None),
                    (Some((label, offset)), _) => (
                        label,
                        Some(i18::try_from(i32::from_str_radix(offset, 10).unwrap()).unwrap()),
                    ),
                    (None, Some((label, offset))) => (
                        label,
                        Some(i18::try_from(-i32::from_str_radix(offset, 10).unwrap()).unwrap()),
                    ),
                };

                let offset = offset.unwrap_or(0i16.into());

                match label {
                    "LOK" => Operand::Lok { offset },
                    label => Operand::Symbol {
                        label: String::from(label),
                        offset,
                    },
                }
            }
        }
    }

    fn apply_offset(address: u18, offset: i18) -> u18 {
        if offset > i18::new(0) {
            address + u18::try_from(u32::try_from(i32::try_from(offset).unwrap()).unwrap()).unwrap()
        } else {
            address
                - u18::try_from(u32::try_from(-i32::try_from(offset).unwrap()).unwrap()).unwrap()
        }
    }

    fn evaluate(&self, address: u18, symbols: &BTreeMap<String, u18>) -> u18 {
        match self {
            Operand::Number { value } => *value,
            Operand::Lok { offset } => Self::apply_offset(address, *offset),
            Operand::Symbol { label, offset } => {
                let symbol = *symbols
                    .get((&label).trim())
                    .unwrap_or_else(|| panic!("Symbol '{}' not found in symbol table", label));
                Self::apply_offset(symbol, *offset)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Code {
    Instruction {
        instruction: Instruction,
        operand: Option<Operand>,
    },
    Data {
        value: Operand,
    },
    AsciiString {
        data: Vec<u8>,
        wide: bool,
    },
}

impl Code {
    fn len(&self) -> u18 {
        match self {
            Self::Instruction { .. } => 1u8.into(),
            Self::Data { .. } => 1u8.into(),
            Self::AsciiString { data, wide: true } => u18::try_from(data.len() as u32).unwrap(),
            Self::AsciiString { data, wide: false } => {
                u18::try_from((data.len() as u32 + 1) / 2).unwrap()
            }
        }
    }

    fn encode(&self, address: u18, symbols: &BTreeMap<String, u18>) -> Vec<u18> {
        match self {
            Code::Instruction {
                instruction,
                operand,
            } => {
                let operand = operand
                    .as_ref()
                    .map(|operand| operand.evaluate(address, &symbols))
                    .unwrap_or(u18::new(0));

                let encoded_instruction = instruction.clone().with_operand(operand).encode();

                vec![encoded_instruction]
            }
            Code::Data { value } => {
                let value = value.evaluate(address, &symbols);

                vec![value]
            }
            Code::AsciiString { data, wide } => {
                let bit9 = u18::new(0b1_0000_0000);

                if *wide {
                    data.iter()
                        .enumerate()
                        .map(|(idx, byte)| {
                            u18::from(*byte)
                                | if idx + 1 == data.len() {
                                    bit9
                                } else {
                                    u18::new(0)
                                }
                        })
                        .collect::<Vec<u18>>()
                } else {
                    data.chunks(2)
                        .enumerate()
                        .map(|(idx, byte)| {
                            let upper_byte = u18::from(byte[0]);
                            let lower_byte = u18::from(byte.get(1).map(|x| *x).unwrap_or(32u8));

                            let upper_byte = upper_byte
                                | if (2 * idx) + 1 == data.len() {
                                    bit9
                                } else {
                                    u18::new(0)
                                };

                            let lower_byte = lower_byte
                                | if ((2 * idx) + 1) + 1 == data.len() {
                                    bit9
                                } else {
                                    u18::new(0)
                                };

                            upper_byte << 9 | lower_byte
                        })
                        .collect()
                }
            }
        }
    }
}

#[derive(Debug)]
struct Line {
    // Currently an unused field, but I'd like to include it in error messages in the future
    _source_line: usize,
    code: Option<Code>,
    comment: Option<String>,
    label: Option<String>,
    even_align: bool,
}

#[derive(Debug)]
struct OrgBlock {
    beginning_address: u18,
    lines: Vec<Line>,
}

#[derive(Debug)]
pub struct Program {
    start_address: Operand,
    blocks: Vec<OrgBlock>,
}

pub fn parse(lines: &Vec<&str>) -> Program {
    let mut program = Program {
        start_address: Operand::Number { value: 0u8.into() },
        blocks: vec![],
    };

    for (source_line, &line) in lines.iter().enumerate() {
        let (line, comment) = match line.split_once(";") {
            None => (line, None),
            Some((line, comment)) => (line, Some(comment)),
        };

        match line.split_once(" ") {
            Some(("SADD", address)) => program.start_address = Operand::parse(address),
            Some(("ORG", address)) => {
                let beginning_address = Operand::parse_number(address).unwrap();

                let block = OrgBlock {
                    beginning_address,
                    lines: vec![],
                };

                program.blocks.push(block);
            }
            _ => {
                let (label, line) = match line.strip_prefix(">") {
                    None => (None, line),
                    Some(line) => {
                        let (label, line) = line.split_once(" ").unwrap();
                        (Some(label), line)
                    }
                };

                let even_align = line.trim().starts_with("EVEN");

                let code = match line.split_once(" ") {
                    None if line == "" => None,
                    _ if line.starts_with("EVEN") => None,
                    _ if line.starts_with("END") => None,
                    Some(("DATA", value)) => {
                        let value = Operand::parse(value);
                        Some(Code::Data { value })
                    }
                    Some(("AS", value)) => {
                        let data = value.as_bytes().to_vec();
                        Some(Code::AsciiString { data, wide: false })
                    }
                    Some(("AW", value)) => {
                        let data = value.as_bytes().to_vec();
                        Some(Code::AsciiString { data, wide: true })
                    }
                    None => {
                        let instruction = Instruction::parse_string(line).unwrap_or_else(|| {
                            eprintln!(
                                "Failed to parse instruction on line {}: '{}'",
                                source_line + 1,
                                line
                            );
                            eprintln!("Full context: {:?}", lines.get(source_line));
                            panic!("Unable to parse instruction");
                        });
                        Some(Code::Instruction {
                            instruction,
                            operand: None,
                        })
                    }
                    Some((instruction, operand)) => {
                        let instruction =
                            Instruction::parse_string(instruction).unwrap_or_else(|| {
                                panic!(
                                    "Failed to parse instruction '{}' on line {} with operand '{}'",
                                    instruction, source_line, operand
                                )
                            });
                        let operand = Operand::parse(operand);
                        Some(Code::Instruction {
                            instruction,
                            operand: Some(operand),
                        })
                    }
                };

                let line = Line {
                    _source_line: source_line,
                    code,
                    comment: comment.map(|comment| String::from(comment)),
                    label: label.map(|comment| String::from(comment)),
                    even_align,
                };

                program.blocks.last_mut().unwrap().lines.push(line);
            }
        }
    }

    program
}

pub fn layout(program: &Program, verbose: bool) -> (Vec<Vec<(u18, Code)>>, BTreeMap<String, u18>) {
    let mut symbols: BTreeMap<String, u18> = BTreeMap::new();

    let program_layout: Vec<_> = program
        .blocks
        .iter()
        .map(|block| {
            let mut layout_block: Vec<(u18, Code)> = vec![];
            let mut address = block.beginning_address;

            if verbose {
                logger!("BLOCK: {:0>6o}\n", address);
            }

            for line in &block.lines {
                // Insert zero padding if line must be even-aligned
                if line.even_align && address & u18::new(1) == u18::new(1) {
                    let padding = Code::Data {
                        value: Operand::Number { value: u18::new(0) },
                    };
                    layout_block.push((address, padding));

                    address = address + u18::new(1);
                }

                let mut verbose_line =
                    format!("{: <18}", line.label.as_ref().unwrap_or(&String::from("")));

                if let Some(code) = &line.code {
                    verbose_line.push_str(&format!("{:0>6o} {:?}", address, code));

                    if let Some(label) = &line.label {
                        symbols.insert(label.clone(), address);
                    }

                    layout_block.push((address, code.clone()));

                    address = address + code.len();
                };

                if let Some(comment) = &line.comment {
                    verbose_line.push_str(&format!(";{}", comment));
                }

                if verbose {
                    logger!("{:}", verbose_line);
                }
            }

            if verbose {
                logger!();
            }

            layout_block
        })
        .collect();

    (program_layout, symbols)
}

pub fn encode(
    program_layout: Vec<Vec<(u18, Code)>>,
    symbols: &BTreeMap<String, u18>,
    program: &Program,
) -> memory::Layout {
    memory::Layout {
        start_address: program.start_address.evaluate(u18::new(0), &symbols),
        segments: program_layout
            .iter()
            .map(|block| memory::LayoutSegment {
                start_addr: block[0].0,
                data: block
                    .iter()
                    .flat_map(|(address, code)| code.encode(*address, symbols))
                    .collect(),
            })
            .collect(),
    }
}

pub fn assemble(lines: &Vec<&str>, verbose: bool) -> tape::Tape {
    let program = parse(lines);

    let (program_layout, symbols) = layout(&program, verbose);

    if verbose {
        logger!("LABELS:\n");

        for (label, address) in &symbols {
            logger!("\t{: <18}{:0>6o}", label, address);
        }
    }

    let memory_layout = encode(program_layout, &symbols, &program);

    memory_layout.encode_76()
}
