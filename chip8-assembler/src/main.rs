use chip8_assembler::*;
use io::{BufWriter, Write};
use std::env;
use std::io::Result;
use std::io::{self, BufRead, BufReader};
use std::{collections::HashMap, fs::File};

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();

    if args.len() != 3 {
        eprintln!("Usage: {} <input asm> <output binary>", args[0]);
        return Ok(());
    }

    let mut address: u16 = 0x202;
    let mut rom: [u8; 0x1000 - 0x300] = [0; 0x1000 - 0x300];

    let mut labels = HashMap::new();
    let mut instructions = HashMap::new();
    let mut entrypoint = String::from("_start");

    labels.insert(".", address.into());

    let bf = BufReader::new(File::open(&args[1])?);
    let mut br = BufWriter::new(File::create(&args[2])?);
    let lines = bf.lines().map(|x| x.unwrap()).collect::<Vec<_>>();
    // parse the file into an intermediate parsed state,
    // so i can parse expressions when all labels and constants
    // are known.
    for line in lines.iter() {
        let rom_addr = address - 0x200;
        if rom_addr > rom.len() as u16 {
            eprintln!("ROM exhausted");
            return Ok(());
        }
        let stripped_line = strip_ws_comments(&line);
        if stripped_line.is_empty() {
            continue;
        }
        if let Some(name) = misc::label(stripped_line) {
            labels.insert(name, address.into());
            continue;
        }
        if let Some((name, value)) = misc::constant(stripped_line) {
            labels.insert(name, value);
            continue;
        }

        if let Some((what, how_many)) = directives::repeat(stripped_line) {
            let (value, did_overflow) = how_many.overflowing_add(rom_addr);
            if value > rom.len() as u16 || did_overflow {
                eprintln!("Not enough ROM to fit in {:x} {} times", what, how_many);
                return Ok(());
            }
            for i in rom_addr..rom_addr + how_many {
                rom[i as usize] = what;
            }
            address = value + 0x200;
            labels.entry(".").and_modify(|x| *x = address.into());

            continue;
        }
        if let Some(how_much) = directives::reserve(stripped_line) {
            let (value, did_overflow) = how_much.overflowing_add(rom_addr);
            if did_overflow || value > rom.len() as u16 {
                eprintln!("Not enough ROM to reserve {} bytes", how_much);
                return Ok(());
            }
            labels.entry(".").and_modify(|x| *x = address.into());
            address = value + 0x200;
            continue;
        }
        if let Some(new_ep) = directives::entrypoint(stripped_line) {
            entrypoint.clear();
            entrypoint.push_str(new_ep);
            continue;
        }
        if let Some(sequence) = directives::sequence_bytes(stripped_line) {
            if sequence.len() > std::u16::MAX as usize {
                eprintln!("Sequence sizes must be in u16 range");
                return Ok(());
            }
            let (value, did_overflow) = rom_addr.overflowing_add(sequence.len() as u16);
            if did_overflow || value > rom.len() as u16 {
                eprintln!("Not enough ROM to fit {} bytes", sequence.len());
                return Ok(());
            }
            for v in sequence {
                rom[address as usize - 0x200] = v;
                address += 1;
            }
            labels.entry(".").and_modify(|x| *x = address.into());
            continue;
        }
        if let Some(i) = instructions::any(stripped_line) {
            instructions.insert(address, i);
            address += 2;
            labels.entry(".").and_modify(|x| *x = address.into());
            continue;
        }
        eprintln!("Unknown line: {:?}", line);
        return Ok(());
    }

    // now I can safely re-parse the instructions.
    // first, pre-parse any partial expressions which were leaning around
    // and convert them to constant so I don't have to re-parse every time.
    let mut labels = labels
        .iter()
        .filter_map(|(a, b)| {
            let b = b.consume(&labels)?;
            Some((*a, b.into()))
        })
        .collect::<HashMap<_, _>>();

    // now, insert the instructions
    for (addr, i) in instructions.iter() {
        labels.entry(".").and_modify(|x| *x = (*addr).into());
        match i.compile(&labels) {
            Some(v) => {
                rom[*addr as usize - 0x200] = (v >> 8) as u8;
                rom[*addr as usize - 0x200 + 1] = (v & 0xff) as u8;
            }
            None => {
                eprintln!(
                    "Couldn't compile instruction: {:?} unknown in constant expression",
                    i
                );
                return Ok(());
            }
        }
    }

    if let Some(entrypoint) = labels
        .get(entrypoint.as_str())
        .and_then(|x| x.consume(&labels))
    {
        rom[0] = (entrypoint >> 8) as u8;
        rom[1] = (entrypoint & 0xff) as u8;
        rom[0] |= 0x10;
    } else {
        eprintln!("Entrypoint {:?} expected to be present. You can change at any time what the entrypoint label is by using '.entrypoint <entrypoint>'", entrypoint);
        return Ok(());
    }

    br.write_all(&rom[..address as usize - 0x200])
}

fn strip_ws_comments(line: &str) -> &str {
    let mut end_offt = line.len();
    if let Some(i) = line.find(';') {
        end_offt = i;
    }

    &line[..end_offt].trim()
}
