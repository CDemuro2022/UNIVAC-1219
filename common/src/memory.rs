use crate::arith;
use crate::instruction::Instruction;
use crate::tape::{self, split_bioctal, Tape};
use std::iter::Peekable;
use ux::{u18, u6};

pub fn load() -> Vec<u18> {
    // The default memory image provided with Duane's emulator
    let data = include_str!("../../data/MEMORY.0");

    let mut memory_image = data
        .lines()
        .filter_map(|value| {
            if value.is_empty() {
                None
            } else {
                let value = value.trim().parse::<u32>().unwrap();
                let value = u18::try_from(value).unwrap();
                Some(value)
            }
        })
        .collect::<Vec<u18>>();

    let line_cnt = memory_image.len();
    logger!("Loaded {}kW of memory\n", line_cnt / 1024);

    // Extend the memory to 40kW to match the actual machine
    memory_image.resize(40 * 1024, u18::new(0));

    memory_image
}

// Prints the memory contents for a given range of memory
pub fn print(memory: &Vec<u18>, low_addr: usize, high_addr: usize) {
    const CHUNK_SIZE: usize = 8;
    for (idx, line) in memory[low_addr..high_addr].chunks(CHUNK_SIZE).enumerate() {
        print!("{:0>6o}\t| ", idx * CHUNK_SIZE + low_addr);

        for item in line.iter() {
            print!("{:0>6o} ", item);
        }

        logger!();
    }

    logger!();
}

// Prints the memory contents parsed as instructions for a given range of memory
pub fn print_instructions(memory: &Vec<u18>, low_addr: usize, high_addr: usize) {
    for (idx, code) in memory[low_addr..high_addr].iter().enumerate() {
        let addr = idx + low_addr;

        match Instruction::parse(*code) {
            Some(instruction) => {
                logger!("{:0>6o}\t| {:?}", addr, instruction);
            }
            None => {
                let code: u32 = Into::into(*code);
                match char::from_u32(code) {
                    Some(ascii) if code >= 32 && code <= 127 => {
                        logger!("{:0>6o}\t| {:0>6o} | {:?}", addr, code, ascii);
                    }
                    _ => {
                        logger!("{:0>6o}\t| {:0>6o}", addr, code);
                    }
                }
            }
        }
    }

    logger!();
}

// The .76 format allows multiple memory segments to be provided on the same tape
pub struct LayoutSegment {
    pub start_addr: u18,
    pub data: Vec<u18>,
}

impl LayoutSegment {
    pub fn encode_76(&self) -> Tape {
        let mut output = vec![];

        let segment_length = u18::new(u32::try_from(self.data.len()).unwrap());
        let segment_start = self.start_addr;
        let segment_end = self.start_addr + segment_length - u18::new(1);

        // Push 76 header
        output.push(u6::new(0o76));
        output.push(u6::try_from((segment_start >> 9) & u18::new(0o77)).unwrap());
        output.push(u6::try_from((segment_start >> 3) & u18::new(0o77)).unwrap());
        output.push(
            u6::try_from(
                ((segment_start & u18::new(0o7)) << 3) | ((segment_end >> 12) & u18::new(0o7)),
            )
            .unwrap(),
        );
        output.push(u6::try_from((segment_end >> 6) & u18::new(0o77)).unwrap());
        output.push(u6::try_from((segment_end >> 0) & u18::new(0o77)).unwrap());

        // Push the contents of the block
        let mut checksum = u18::new(0);
        for word in &self.data {
            checksum = arith::add_18(checksum, *word);
            output.extend(split_bioctal(word));
        }

        // Block checksum: ones-complement sum of all data words
        output.extend(split_bioctal(&checksum));

        output
    }

    pub fn decode_76(tape: &mut Peekable<impl Iterator<Item = u6>>) -> Self {
        let header = tape.next();

        if let Some(0o76) = header.map(|x| u8::from(x)) {
            let b0: u32 = tape.next().unwrap().into();
            let b1: u32 = tape.next().unwrap().into();
            let b2: u32 = tape.next().unwrap().into();
            let b3: u32 = tape.next().unwrap().into();
            let b4: u32 = tape.next().unwrap().into();

            let start_addr = (b0 << 9) | (b1 << 3) | (b2 >> 3);
            let end_addr = ((b2 & 0b111) << 12) | (b3 << 6) | b4;

            let mut words = vec![];

            for _addr in start_addr..=end_addr {
                words.push(tape::join_bioctal(tape));
            }

            // Consume block checksum
            let _checksum = tape::join_bioctal(tape);

            LayoutSegment {
                start_addr: u18::try_from(start_addr).unwrap(),
                data: words,
            }
        } else {
            panic!("UNEXPECTED {:?}", header);
        }
    }

    pub fn end_addr(&self) -> u18 {
        let length = self.data.len();

        self.start_addr + u18::new(length as u32) - u18::new(1)
    }

    pub fn get(&self, idx: u18) -> u18 {
        let idx = u32::from(idx - self.start_addr) as usize;

        self.data[idx]
    }

    pub fn enumerate(&self) -> impl Iterator<Item = (u18, u18)> {
        self.data.iter().enumerate().map(|(idx, data)| {
            let addr = u18::new(idx as u32) + self.start_addr;
            (addr, *data)
        })
    }
}

pub struct Layout {
    pub segments: Vec<LayoutSegment>,
    pub start_address: u18,
}

impl Layout {
    // A discussion on loading programs on the Univac:
    //
    // There are three stages to loading programs. The first stage loader at
    // address 0o500, the second stage loader at address 0o540, then the LECPAC
    // loader.
    //
    // First stage "PT BOOT" loader: Written to address 0o500. Does not
    // understand .76 format. Simply reads from channel 0, skips leading 0s, and
    // reads exactly 97 18 bit words to address 0o536-0o676. After loading, STOP
    // 1, then jumps to 0o540.
    //
    // These are the instructions that are taped on the Univac that you manually
    // need to key into memory to bootstrap our program loading.
    //
    // PT BOOT — Paper Tape Bootstrap (0o500-0o534)
    //
    // 000500  503400  SIL
    // 000501  507310  ENTSR 8
    // 000502  507201  ENTICR 1
    // 000503  501300  EXF 0
    // 000504  000534    (TACW)
    // 000505  000534    (IACW)
    // 000506  360000  ENTBK 0
    // 000507  420003  STRB 3
    // 000510  700002  ENTALK 2
    // 000511  440004  STRAL 4
    // 000512  700000  ENTALK 0
    // 000513  501100  IN 0
    // 000514  000002    (TACW)
    // 000515  000002    (IACW)
    // 000516  502100  SKPIIN 0
    // 000517  340516  JP 516
    // 000520  504606  LSHAL 6
    // 000521  510002  SLSET 2
    // 000522  630525  JPALNZ 525
    // 000523  100003  ENTAU 3
    // 000524  600513  JPAUZ 513
    // 000525  570004  ISK 4
    // 000526  340513  JP 513
    // 000527  450536  STRALB 536
    // 000530  560535  BSK 535
    // 000531  340507  JP 507
    // 000532  505601  STOP 1
    // 000533  340540  JP 540
    // 000534  000151    (data: EF 105)
    //
    // PT BOOT can launch .76+ tapes. A .76+ tape's first 97 words are a second
    // stage loader that is loaded by PT BOOT and jumped to at 0o540. The second
    // stage loader can load the .76 format. It reads the remaining input of the
    // .76+ tape as a normal .76 formatted tape.
    //
    // MEMORY.0 dump has this second stage loader already loaded to 0o540.
    // That's why our emu begins execution at this address.
    //
    // The behavior of the 0o540 second stage loader is as follows: scans for
    // 0o76 segments, verifies per-segment checksums, reads the 0o77 header to
    // get the start address, STOP 1, then jumps to start address. STOP 5 on
    // checksum failure. Ignores all 0s that occur outside of segments.
    //
    // LECPAC is a general purpose utility program that also includes a program
    // loader. Its loader has the following behavior: the tape must have begin
    // with at least 1 lead-in 0. Scans for 0o76 segments, verifies per-segment
    // checksums, stops on >=4 consecutive zeros outside of segments. Does not
    // support 0o77 header for start address. STOP 5 with AU=0 AL=0 on success,
    // AU=computed AL=tape checksum on failure.
    //
    // On real hardware, what we do is to manually key in PT BOOT, start
    // executing at 0o500, then send a .76+ LECPAC tape. From then on, when we
    // need to load a program, we jump to the LECPAC loading address and send
    // our .76 files through.
    //
    // This encode_76 function encodes a .76 tape that is compatible with both
    // the second stage 0o540 loader and the LECPAC loader.
    pub fn encode_76(self) -> Tape {
        // One lead-in 0 for LECPAC. 0o540 loader ignores this.
        let mut out = vec![u6::new(0)];

        for segment in &self.segments {
            out.append(&mut segment.encode_76());
        }

        // 4 zeros marks end-of-tape for LECPAC
        out.extend([u6::new(0); 4]);

        // 0o77 header with start address (used by 0o540 loader, LECPAC never
        // reads this)
        out.append(&mut split_bioctal(&u18::new(0o77)));
        out.append(&mut split_bioctal(&self.start_address));

        out
    }

    pub fn decode_76(tape: &mut Peekable<impl Iterator<Item = u6>>) -> Self {
        // Parse the tape into segments
        let mut segments = vec![];
        let mut start_address = u18::new(0);

        while tape.peek().is_some() {
            // Ignore characters until a new section header is found
            while let Some(x) = tape.peek().map(|x| u8::try_from(*x).unwrap()) {
                match x {
                    // Skip lead-in zeroes
                    0o00 => {
                        tape.next();
                    }
                    // Start parsing if we see a 76 header
                    0o76 => break,
                    // Parse the start address if we see a 77 header
                    0o77 => {
                        let _ = tape.next();
                        start_address = tape::join_bioctal(tape);
                    }
                    // Otherwise print the characters
                    x => {
                        logger!("UNEXPECTED {:02o}", x);
                        tape.next();
                    }
                }
            }

            if tape.peek().is_some() {
                segments.push(LayoutSegment::decode_76(tape));
            }
        }

        Layout {
            segments,
            start_address,
        }
    }
}
