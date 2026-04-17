use std::fs;
use std::io::{IsTerminal, Read, Write};
use std::sync::mpsc;
use std::time::SystemTime;
use std::{thread, time};
use ux::{u5, u6, u18};

use common::{assembler::assemble, emulator::State, memory, tape};

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The .76, .76+, or .BIN file to mount
    #[arg(short, long, value_name = "FILE")]
    tape_file: Option<String>,

    /// The UNIVAC assembly program to assemble and run
    #[arg(short, long, value_name = "FILE")]
    source_file: Option<String>,

    /// The memory address to start from. (Default 000540)
    #[arg(short = 'a', long, value_name = "ADDRESS")]
    start_address: Option<String>,

    /// Which stop switches to set
    #[arg(long, value_name = "INT")]
    stop: Vec<usize>,

    /// Which skip switches to set
    #[arg(long, value_name = "INT")]
    skip: Vec<usize>,

    /// Temporarily pauses the emulator to make it run at the same speed as the real hardware
    #[arg(short, long)]
    realtime: bool,

    #[arg(short, long)]
    verbose: bool,
}

fn get_non_blocking_stdin() -> mpsc::Receiver<u8> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        loop {
            let in_byte = if std::io::stdin().is_terminal() {
                // Interactive terminal: use console for char-by-char input
                let in_char = console::Term::stdout().read_char().unwrap();

                let mut buf = [0u8; 10];
                let _ = in_char.encode_utf8(&mut buf);
                buf[0]
            } else {
                // Piped input: read from stdin
                let mut byte = [0u8; 1];
                std::io::stdin().read_exact(&mut byte).unwrap();
                byte[0]
            };

            tx.send(in_byte).unwrap();
        }
    });

    rx
}

fn write_to_terminal(out: &str) {
    print!("{:}", out);
    std::io::stdout().flush().unwrap();
}

fn main() {
    let args = Args::parse();

    let tape = match (args.tape_file, args.source_file) {
        (None, None) => tape::new(),
        (Some(path), None) => {
            if path.ends_with(".76") || path.ends_with(".76+") {
                let data = fs::read_to_string(path).unwrap();
                tape::deserialize_bioctal(&data)
            } else {
                let data = fs::read(path).unwrap().into_iter().collect::<Vec<_>>();
                tape::deserialize_bin(&data)
            }
        }
        (None, Some(path)) => {
            let data = fs::read_to_string(path).unwrap();
            let lines = data.lines().collect::<Vec<_>>();

            assemble(&lines, false)
        }
        (Some(_), Some(_)) => tape::new(),
    };

    let start_address = match args.start_address {
        None => u18::from(0o0540u16),
        Some(start_address) => {
            u18::try_from(u32::from_str_radix(&start_address, 8).unwrap()).unwrap()
        }
    };

    let mut stop = u6::new(0);
    let mut skip = u5::new(0);

    for stop_switch in args.stop {
        stop |= u6::new(1 << stop_switch)
    }

    for skip_switch in args.skip {
        skip |= u5::new(1 << skip_switch)
    }

    let mut emulator = State::new(memory::load(), tape);
    emulator.p = start_address;
    emulator.running = true;
    emulator.stop = stop;
    emulator.skip = skip;

    let start_time = SystemTime::now();

    let stdin = get_non_blocking_stdin();

    while emulator.running {
        emulator.step(args.verbose, write_to_terminal);

        let delay_millis = 10_000u64;

        // Only occasionally check IO or machine time to keep the emulator a tight loop
        if emulator.time_microseconds % delay_millis < 10 {
            let now = SystemTime::now();
            let wall_clock_time = now.duration_since(start_time).unwrap().as_micros();

            if args.realtime
                && (wall_clock_time + u128::from(delay_millis))
                    < u128::from(emulator.time_microseconds)
            {
                thread::sleep(time::Duration::from_micros(delay_millis));
            }

            if let Ok(char) = stdin.try_recv() {
                emulator
                    .io
                    .input_async(&mut emulator.memory, char, write_to_terminal);
            }
        }
    }
}
