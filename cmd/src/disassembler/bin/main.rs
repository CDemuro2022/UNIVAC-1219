use clap::Parser;
use common::memory::LayoutSegment;
use common::{instruction::Instruction, memory, tape};
use std::{collections::BTreeMap, collections::BTreeSet, fs};
use ux::u18;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    source_file: String,
}

fn str_of_word(word: u18) -> Option<String> {
    let word = u32::from(word);

    if word >= 32 && word < 127 {
        Some(String::from(char::from_u32(word).unwrap()))
    } else if word == 10 {
        Some(String::from("\n"))
    } else if word == 13 {
        Some(String::from("\\r"))
    } else {
        None
    }
}

fn main() {
    let args = Args::parse();

    let path = args.source_file;

    println!("Disassembling {}...\n", path);

    let tape = if path.ends_with(".76") || path.ends_with(".76+") {
        let data = fs::read_to_string(path).unwrap();
        tape::deserialize_bioctal(&data)
    } else {
        let data = fs::read(path).unwrap();
        tape::deserialize_bin(&data)
    };

    let mut tape_streamable = tape.into_iter().peekable();

    let layout = memory::Layout::decode_76(&mut tape_streamable);

    let mut jump_points = BTreeSet::new();
    let mut return_jump_points = BTreeSet::new();
    let mut return_points = BTreeSet::new();
    let mut data_points = BTreeSet::new();
    let mut array_points = BTreeSet::new();

    for LayoutSegment {
        start_addr,
        data: words,
    } in &layout.segments
    {
        for (offset, word) in words.iter().enumerate() {
            let addr = *start_addr + u18::try_from(u32::try_from(offset).unwrap()).unwrap();
            let addr_upper = addr & u18::new(0o770000);

            if let Some(instruction) = Instruction::parse(ux::u18::try_from(*word).unwrap()) {
                match instruction {
                    Instruction::JP { y }
                    | Instruction::JPAUZ { y }
                    | Instruction::JPALZ { y }
                    | Instruction::JPAUNZ { y }
                    | Instruction::JPALNZ { y }
                    | Instruction::JPAUP { y }
                    | Instruction::JPALP { y }
                    | Instruction::JPAUNG { y }
                    | Instruction::JPALNG { y } => {
                        jump_points.insert(addr_upper | u18::from(y));
                    }
                    Instruction::RJP { y } => {
                        return_jump_points.insert(addr_upper | u18::from(y));
                    }
                    Instruction::IJP { y } => {
                        return_points.insert(addr_upper | u18::from(y));
                    }
                    Instruction::CMAL { y }
                    | Instruction::SLSU { y }
                    | Instruction::CMSK { y }
                    | Instruction::ENTAU { y }
                    | Instruction::ENTAL { y }
                    | Instruction::ADDAL { y }
                    | Instruction::SUBAL { y }
                    | Instruction::ADDA { y }
                    | Instruction::SUBA { y }
                    | Instruction::MULAL { y }
                    | Instruction::DIVA { y }
                    | Instruction::ENTB { y }
                    | Instruction::ENTBK { u: y }
                    | Instruction::CL { y }
                    | Instruction::STRB { y }
                    | Instruction::STRAL { y }
                    | Instruction::STRAU { y }
                    | Instruction::SLSET { y }
                    | Instruction::SLCL { y }
                    | Instruction::SLCP { y }
                    | Instruction::BSK { y }
                    | Instruction::ISK { y }
                    | Instruction::ENTALK { u: y }
                    | Instruction::ADDALK { u: y }
                    | Instruction::STRICR { y }
                    | Instruction::STRADR { y }
                    | Instruction::STRSR { y } => {
                        // It's hard to tell sometimes whether we're parsing an instruction in the memory image
                        // or just a constant that looks like an instruction. It's probably worth filtering these
                        // entries on some heuristic (like what memory range we fine the word in)
                        data_points.insert(addr_upper | u18::from(y));
                    }

                    Instruction::CMALB { y }
                    | Instruction::SLSUB { y }
                    | Instruction::CMSKB { y }
                    | Instruction::ENTAUB { y }
                    | Instruction::ENTALB { y }
                    | Instruction::ADDALB { y }
                    | Instruction::SUBALB { y }
                    | Instruction::ADDAB { y }
                    | Instruction::SUBAB { y }
                    | Instruction::MULALB { y }
                    | Instruction::DIVAB { y }
                    | Instruction::ENTBB { y }
                    | Instruction::ENTBKB { u: y }
                    | Instruction::CLB { y }
                    | Instruction::STRBB { y }
                    | Instruction::STRALB { y }
                    | Instruction::STRAUB { y } => {
                        array_points.insert(addr_upper | u18::from(y));
                    }

                    _ => {}
                }
            }
        }
    }

    // Look for places referred to by an RJP and IJP (return) instruction
    let subroutines = return_jump_points
        .union(&return_points)
        .into_iter()
        .enumerate()
        .map(|(idx, x)| (*x, format!("FUNC_{:}", idx)))
        .collect::<BTreeMap<u18, String>>();

    let data = data_points
        .iter()
        .enumerate()
        .map(|(idx, x)| (*x, format!("DATA_{:}", idx)))
        .collect::<BTreeMap<u18, String>>();

    let arrays = array_points
        .iter()
        .enumerate()
        .map(|(idx, x)| (*x, format!("ARRAY_{:}", idx)))
        .collect::<BTreeMap<u18, String>>();

    println!("START ADDRESS:");
    println!("  {:06o}\n", layout.start_address);

    println!("SUBROUTINES:");
    for (addr, name) in &subroutines {
        println!("  {:}\t{:06o}", name, addr)
    }
    println!();

    println!("DATA:");
    for (addr, name) in &data {
        println!("  {:}\t{:06o}", name, addr)
    }
    println!();

    println!("ARRAY:");
    for (addr, name) in &arrays {
        println!("  {:}\t{:06o}", name, addr)
    }
    println!();

    println!("IMAGE:");

    // Print out the segments
    for LayoutSegment {
        start_addr,
        data: words,
    } in &layout.segments
    {
        let end_addr = *start_addr + u18::try_from(u32::try_from(words.len()).unwrap()).unwrap();
        println!("{:06o} -> {:06o}\n", start_addr, end_addr);

        let mut words = words.into_iter().enumerate().peekable();

        while let Some((offset, word)) = words.next() {
            let addr = *start_addr + u18::try_from(u32::try_from(offset).unwrap()).unwrap();

            if addr == layout.start_address {
                println!("\nSTART");
            } else if let Some(name) = subroutines.get(&addr) {
                println!("\n{:}", name);
            } else if let Some(name) = data.get(&addr) {
                println!("\n{:}", name);
            }

            if let Some(str) = str_of_word(*word) {
                let mut str = str;

                loop {
                    if let Some((_, word)) = words.peek() {
                        // Mask out bit 8, since it's sometimes used to mark the end of a string
                        if let Some(new_str) = str_of_word(**word & u18::new(0o777377)) {
                            words.next();
                            str.push_str(&new_str);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                print!("\n  {:06o}: STRING '", addr);

                // Print the string with line-feeds and consistent indentation to make
                // ASCII-art easier to read
                for char in str.chars() {
                    if char == '\n' {
                        println!("\\n");
                        print!("                 >")
                    } else {
                        print!("{:}", char)
                    }
                }

                print!("'\n")
            } else {
                if jump_points.contains(&addr) {
                    print!("> ");
                } else {
                    print!("  ");
                }

                print!("{:06o}: {:06o}", addr, word);

                let addr_upper = addr & u18::new(0o770000);

                if let Some(instruction) = Instruction::parse(*word) {
                    match instruction {
                        Instruction::JP { y }
                        | Instruction::JPAUZ { y }
                        | Instruction::JPALZ { y }
                        | Instruction::JPAUNZ { y }
                        | Instruction::JPALNZ { y }
                        | Instruction::JPAUP { y }
                        | Instruction::JPALP { y }
                        | Instruction::JPAUNG { y }
                        | Instruction::JPALNG { y } => {
                            let target = addr_upper | u18::from(y);

                            if addr > target && addr - target < u18::new(10) {
                                print!(" {:} LOK-{:}", instruction.name(), addr - target)
                            } else if target > addr && target - addr < u18::new(10) {
                                print!(" {:} LOK+{:}", instruction.name(), target - addr)
                            } else {
                                print!(" {:} {:06o}", instruction.name(), target)
                            }
                        }
                        Instruction::RJP { y } | Instruction::IJP { y } => {
                            let target = addr_upper | u18::from(y);

                            if let Some(name) = subroutines.get(&target) {
                                print!(" {:} {:}", instruction.name(), name)
                            } else {
                                print!(" {:?}", instruction)
                            }
                        }

                        Instruction::CMAL { y }
                        | Instruction::SLSU { y }
                        | Instruction::CMSK { y }
                        | Instruction::ENTAU { y }
                        | Instruction::ENTAL { y }
                        | Instruction::ADDAL { y }
                        | Instruction::SUBAL { y }
                        | Instruction::ADDA { y }
                        | Instruction::SUBA { y }
                        | Instruction::MULAL { y }
                        | Instruction::DIVA { y }
                        | Instruction::ENTB { y }
                        | Instruction::ENTBK { u: y }
                        | Instruction::CL { y }
                        | Instruction::STRB { y }
                        | Instruction::STRAL { y }
                        | Instruction::STRAU { y }
                        | Instruction::SLSET { y }
                        | Instruction::SLCL { y }
                        | Instruction::SLCP { y }
                        | Instruction::BSK { y }
                        | Instruction::ISK { y }
                        | Instruction::ENTALK { u: y }
                        | Instruction::ADDALK { u: y }
                        | Instruction::STRICR { y }
                        | Instruction::STRADR { y }
                        | Instruction::STRSR { y } => {
                            let target = addr_upper | u18::from(y);

                            if let Some(name) = data.get(&target) {
                                print!(" {:} {:}", instruction.name(), name)
                            } else {
                                print!(" {:?}", instruction)
                            }
                        }

                        _ => print!(" {:?}", instruction),
                    }
                }
            }

            println!();
        }
        println!("\n");
    }
}
