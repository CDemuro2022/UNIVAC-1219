use clap::Parser;
use common::assembler;
use common::tape;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;

mod serial;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    source_file: String,

    #[arg(short, long, value_name = "FILE")]
    output_file: Option<String>,

    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    let path = args.source_file;

    if args.verbose {
        println!("Assembling {}...\n", path);
    }

    let data = fs::read_to_string(&path).unwrap();
    let lines = data.lines().collect::<Vec<_>>();

    let tape = assembler::assemble(&lines, args.verbose);

    let usb_serial_ports = match serialport::available_ports() {
        Err(_) => BTreeSet::new(),
        Ok(ports) => ports
            .into_iter()
            .filter_map(|port| {
                if port.port_name.contains("tty.usb") {
                    Some(port.port_name)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<String>>(),
    };

    match args.output_file {
        Some(output_path) if usb_serial_ports.contains(&output_path) => {
            serial::transmit(&tape, output_path, 0.001)
        }
        _ => {
            let output_path = args.output_file.unwrap_or({
                let path = path.clone();
                path.strip_suffix(".TXT")
                    .map(|path| path.to_owned())
                    .unwrap_or(path)
                    + ".76"
            });

            let mut out_file = fs::File::create(&output_path).unwrap();

            if output_path.ends_with(".76") {
                write!(out_file, "{:}", tape::serialize_bioctal(&tape)).unwrap()
            } else {
                out_file.write_all(&tape::serialize_bin(&tape)).unwrap()
            }
        }
    }
}
