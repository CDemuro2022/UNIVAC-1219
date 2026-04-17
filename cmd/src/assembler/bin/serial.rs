use std::io::Write;
use std::thread;
use std::time::Duration;

use indicatif::ProgressBar;

use common::tape::{self, Tape};

pub fn transmit(tape: &Tape, port_name: String, delay_secs: f64) -> () {
    let delay = Duration::from_secs_f64(delay_secs);

    // ── Open serial port at 9600 8N1 ─────────────────────────────────────────
    let mut port = serialport::new(&port_name, 9_600)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(10))
        .open()
        .unwrap_or_else(|e| {
            eprintln!("ERROR opening {}: {}", port_name, e);
            std::process::exit(1);
        });

    println!("Opened {} at 9600 8N1. Uploading...", port_name);

    let tape_bin = tape::serialize_bin(&tape);

    let progress_bar = ProgressBar::new(tape_bin.len() as u64);

    for value in &tape_bin {
        // Send the byte
        port.write_all(&[*value]).unwrap_or_else(|e| {
            eprintln!("ERROR writing to serial port: {}", e);
            std::process::exit(1);
        });
        port.flush().unwrap_or_else(|e| {
            eprintln!("WARNING: flush error: {}", e);
        });

        progress_bar.inc(1);

        if delay.as_nanos() > 0 {
            thread::sleep(delay);
        }
    }

    progress_bar.finish_with_message("done");

    println!("\nSent {} bytes to {}.", tape_bin.len(), port_name);
}
