#![feature(iter_map_while)]
#![feature(fmt_internals)]
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    convert::TryFrom,
    fmt::{Display, Formatter, Result},
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};
#[derive(Clone, Copy)]
enum Argument {
    Constant(u16),
    Register(u16),
}

impl<'a> std::fmt::Display for Argument {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Constant(v) => write!(f, "\x1b[38;5;174m{:x}\x1b[m", v),
            Self::Register(v) => write!(f, "\x1b[38;5;208mV{:1X}\x1b[m", v),
        }
    }
}

impl<'a> Argument {
    #[inline]
    fn value(&self) -> u16 {
        match self {
            Self::Constant(v) => *v,
            Self::Register(v) => *v,
        }
    }
}

#[derive(Clone, Copy)]
enum Instruction {
    Load {
        what: Argument,
        into: Argument,
    },
    Add {
        what: Argument,
        into: Argument,
    },
    Sub {
        what: Argument,
        into: Argument,
        inverted: bool,
    },
    And(Argument, Argument),
    Or(Argument, Argument),
    Xor(Argument, Argument),
    LoadI(Argument),
    AddI(Argument),
    LoadR(Argument),
    Dump(Argument),
    Draw(Argument, Argument, Argument),
    Call(Argument),
    Jump {
        target: Argument,
        adds_v0: bool,
    },
    Ret,
    Clear,
    SkipValue {
        register: Argument,
        what: Argument,
        is_negated: bool,
    },
    SkipKey {
        register: Argument,
        is_negated: bool,
    },
    LoadKey(Argument),
    LoadDelay(Argument),
    SetSound(Argument),
    SetDelay(Argument),
    Shift {
        what: Argument,
        into: Argument,
        is_left: bool,
    },
    Bcd(Argument),
    Font(Argument),
    Random(Argument, Argument),
}

impl Instruction {
    pub fn is_call(&self) -> bool {
        matches!(self, Instruction::Call(_))
    }
    pub fn from_opcode(opcode: u16) -> Option<Self> {
        // AXBC
        let (a, x, b, c) = (
            opcode >> 12,
            opcode >> 8 & 0xf,
            opcode >> 4 & 0xf,
            opcode & 0xf,
        );

        let bc = b << 4 | c;

        let value = match (a, b, c) {
            (0, 0xe, 0) => Self::Clear,
            (0, 0xe, 0xe) => Self::Ret,
            (1, _, _) => Self::Jump {
                target: Argument::Constant(opcode & 0xfff),
                adds_v0: false,
            },
            (2, _, _) => Self::Call(Argument::Constant(opcode & 0xfff)),
            (3, _, _) => Self::SkipValue {
                register: Argument::Register(x),
                is_negated: false,
                what: Argument::Constant(bc),
            },
            (4, _, _) => Self::SkipValue {
                register: Argument::Register(x),
                what: Argument::Constant(bc),
                is_negated: true,
            },
            (5, y, 0) => Self::SkipValue {
                register: Argument::Register(x),
                what: Argument::Register(y),
                is_negated: false,
            },
            (6, _, _) => Self::Load {
                what: Argument::Constant(bc),
                into: Argument::Register(x),
            },
            (7, _, _) => Self::Add {
                what: Argument::Constant(bc),
                into: Argument::Register(x),
            },
            (8, y, 0) => Self::Load {
                what: Argument::Register(y),
                into: Argument::Register(x),
            },
            (8, y, 1) => Self::Or(Argument::Register(x), Argument::Register(y)),
            (8, y, 2) => Self::And(Argument::Register(x), Argument::Register(y)),
            (8, y, 3) => Self::Xor(Argument::Register(x), Argument::Register(y)),
            (8, y, 4) => Self::Add {
                what: Argument::Register(y),
                into: Argument::Register(x),
            },
            (8, y, 5) => Self::Sub {
                what: Argument::Register(x),
                into: Argument::Register(y),
                inverted: false,
            },
            (8, y, 6) => Self::Shift {
                what: Argument::Register(x),
                into: Argument::Register(y),
                is_left: false,
            },
            (8, y, 7) => Self::Sub {
                what: Argument::Register(y),
                into: Argument::Register(x),
                inverted: true,
            },
            (8, y, 0xe) => Self::Shift {
                what: Argument::Register(x),
                into: Argument::Register(y),
                is_left: true,
            },
            (9, y, 0) => Self::SkipValue {
                register: Argument::Register(x),
                what: Argument::Register(y),
                is_negated: true,
            },
            (0xa, _, _) => Self::LoadI(Argument::Constant(opcode & 0xfff)),
            (0xb, _, _) => Self::Jump {
                target: Argument::Constant(opcode & 0xfff),
                adds_v0: true,
            },
            (0xc, _, _) => Self::Random(Argument::Register(x), Argument::Constant(bc)),
            (0xd, y, n) => Self::Draw(
                Argument::Register(x),
                Argument::Register(y),
                Argument::Constant(n),
            ),
            (0xe, 9, 0xe) => Self::SkipKey {
                register: Argument::Register(x),
                is_negated: false,
            },
            (0xe, 0xa, 1) => Self::SkipKey {
                register: Argument::Register(x),
                is_negated: true,
            },
            (0xf, 0, 7) => Self::LoadDelay(Argument::Register(x)),
            (0xf, 0, 0xa) => Self::LoadKey(Argument::Register(x)),
            (0xf, 1, 5) => Self::SetDelay(Argument::Register(x)),
            (0xf, 1, 8) => Self::SetSound(Argument::Register(x)),
            (0xf, 1, 0xe) => Self::AddI(Argument::Register(x)),
            (0xf, 2, 9) => Self::Font(Argument::Register(x)),
            (0xf, 3, 3) => Self::Bcd(Argument::Register(x)),
            (0xf, 5, 5) => Self::Dump(Argument::Register(x)),
            (0xf, 6, 5) => Self::LoadR(Argument::Register(x)),
            _ => return None,
        };
        Some(value)
    }

    pub fn name_str(&self) -> &'static str {
        match self {
            Self::Load { into: _, what: _ } => "LD",
            Self::Add { into: _, what: _ } => "ADD",
            Self::Sub {
                into: _,
                what: _,
                inverted,
            } => {
                if *inverted {
                    "SBI"
                } else {
                    "SUB"
                }
            }
            Self::And(_, _) => "AND",
            Self::Or(_, _) => "OR",
            Self::Xor(_, _) => "XOR",
            Self::LoadI(_) => "LDI",
            Self::AddI(_) => "ADDI",
            Self::SetSound(_) => "SND",
            Self::LoadR(_) => "LDR",
            Self::LoadKey(_) => "LDK",
            Self::Dump(_) => "DMP",
            Self::Draw(_, _, _) => "DRW",
            Self::Call(_) => "CALL",
            Self::Jump { adds_v0, target: _ } => {
                if *adds_v0 {
                    "JP0"
                } else {
                    "JP"
                }
            }
            Self::Bcd(_) => "BCD",
            Self::Random(_, _) => "RND",
            Self::SkipKey {
                register: _,
                is_negated,
            } => {
                if *is_negated {
                    "SNK"
                } else {
                    "SIK"
                }
            }
            Self::SkipValue {
                register: _,
                what: _,
                is_negated,
            } => {
                if *is_negated {
                    "SNE"
                } else {
                    "SEQ"
                }
            }
            Self::SetDelay(_) => "DLY",
            Self::LoadDelay(_) => "LDD",
            Self::Ret => "RET",
            Self::Clear => "CLR",
            Self::Font(_) => "FNT",
            Self::Shift {
                what: _,
                into: _,
                is_left,
            } => {
                if *is_left {
                    "SHL"
                } else {
                    "SHR"
                }
            }
        }
    }

    pub fn format_args(
        &self,
        f: &mut Formatter,
        labels: &HashMap<u16, String>,
        sprites: &HashSet<u16>,
    ) -> Result {
        match self {
            Self::Load { what, into }
            | Self::Add { what, into }
            | Self::Sub {
                what,
                into,
                inverted: _,
            }
            | Self::And(into, what)
            | Self::Or(into, what)
            | Self::Xor(into, what)
            | Self::SkipValue {
                register: into,
                what,
                is_negated: _,
            } => write!(f, "{}, {}", into, what),
            Self::AddI(what)
            | Self::LoadDelay(what)
            | Self::SetDelay(what)
            | Self::SetSound(what)
            | Self::LoadR(what)
            | Self::Dump(what)
            | Self::LoadKey(what)
            | Self::Bcd(what)
            | Self::Font(what)
            | Self::SkipKey {
                register: what,
                is_negated: _,
            } => write!(f, "{}", what),
            Self::LoadI(what) => {
                if let Some(name) = sprites.get(&what.value()) {
                    write!(f, "\x1b[38;5;176m@{:x}", name)
                } else {
                    write!(f, "\x1b[38;5;176m{}", what)
                }
            }
            Self::Jump { adds_v0: _, target } | Self::Call(target) => {
                write!(f, "\x1b[38;5;68m")?;
                if let Some(name) = labels.get(&target.value()) {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}", target)
                }
            }
            Self::Random(into, mask) => {
                write!(f, "{}", into)?;
                if mask.value() != 0xff {
                    write!(f, ", {}", mask)?;
                }
                Ok(())
            }
            Self::Draw(a, b, c) => write!(f, "{}, {}, {}", a, b, c),
            Self::Ret | Self::Clear => Ok(()),
            Self::Shift {
                into,
                what,
                is_left: _,
            } => {
                write!(f, "{}", what)?;
                if into.value() != what.value() {
                    write!(f, ", {}", into)
                } else {
                    Ok(())
                }
            }
        }
    }
}

use structopt::StructOpt;

fn read_u16(slice: &[u8]) -> Option<u16> {
    if slice.len() >= 2 {
        let a = slice[0] as u16;
        let b = slice[1] as u16;
        Some(a << 8 | b)
    } else {
        None
    }
}

struct U16Reader<'a>(&'a [u8], u16);

impl<'a> Iterator for U16Reader<'a> {
    type Item = (u16, u16);
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() >= 2 {
            let a = self.0[0] as u16;
            let b = self.0[1] as u16;
            self.0 = &self.0[2..];
            let v = (self.1, a << 8 | b);
            self.1 += 2;
            Some(v)
        } else {
            None
        }
    }
}

impl<'a> U16Reader<'a> {
    /// Gives back another iterator who has the input
    /// from the expected address.
    pub fn starting_from(self, addr: u16) -> Self {
        Self(&self.0[addr as usize - 0x200..], addr)
    }
}

struct Program<'a> {
    labels: HashMap<u16, String>,
    sprites: HashSet<u16>,
    instructions: BTreeMap<u16, (u16, Instruction)>,
    draw_sizes: HashSet<u16>,
    buffer: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for Program<'a> {
    type Error = &'static str;
    fn try_from(buffer: &'a [u8]) -> std::result::Result<Self, Self::Error> {
        let mut first_jump = read_u16(&buffer)
            .and_then(Instruction::from_opcode)
            .filter(|x| {
                matches!(
                    x,
                    Instruction::Jump {
                        adds_v0: false,
                        target: _
                    }
                )
            })
            .map(|x| {
                if let Instruction::Jump { adds_v0: _, target } = x {
                    target.value()
                } else {
                    unreachable!();
                }
            });
        if first_jump.is_none() {
            return Err("Expected a jump, malformed binary.");
        }

        let first_jump = first_jump.take().unwrap();
        let mut label_queue = VecDeque::new();
        let mut labels = HashMap::new();
        let mut instructions = BTreeMap::new();
        let mut sprites = HashSet::new();
        let mut draw_sizes = HashSet::new();
        let generate_label = |is_call: bool, location: u16| {
            if !is_call {
                format!("label@{:x}", location)
            } else {
                format!("function@{:x}()", location)
            }
        };

        let mut visited = HashSet::new();

        labels.insert(first_jump, String::from("main"));
        label_queue.push_back(first_jump);

        while let Some(next_label) = label_queue.pop_front() {
            for (address, opcode, next_op) in U16Reader(&buffer, 0)
                .starting_from(next_label)
                .map_while(|(address, opcode)| {
                    if visited.contains(&address) {
                        return None;
                    }
                    let i = Instruction::from_opcode(opcode)?;
                    visited.insert(address);
                    Some((address, opcode, i))
                })
            {
                match next_op {
                    Instruction::Call(target) | Instruction::Jump { target, adds_v0: _ } => {
                        labels
                            .entry(target.value())
                            .or_insert_with_key(|key| generate_label(next_op.is_call(), *key));
                        label_queue.push_back(target.value());
                    }
                    Instruction::LoadI(what) => {
                        sprites.insert(what.value());
                    }
                    Instruction::Draw(_, _, size) => {
                        draw_sizes.insert(size.value());
                    }
                    _ => {}
                }

                instructions.insert(address, (opcode, next_op));
            }
        }

        Ok(Self {
            instructions,
            labels,
            sprites,
            draw_sizes,
            buffer,
        })
    }
}
impl<'a> Display for Program<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for (addr, (opcode, instruction)) in self.instructions.iter() {
            if let Some(name) = self.labels.get(addr) {
                writeln!(f, "\x1b[38;5;49m{}:\x1b[m", name)?;
            }
            write!(
                f,
                "\x1b[38;5;239m{:04X} \x1b[38;5;236m{:04x} \x1b[38;5;204m{} ",
                addr,
                opcode,
                instruction.name_str()
            )?;
            instruction.format_args(f, &self.labels, &self.sprites)?;
            writeln!(f, "\x1b[m")?;
        }

        Ok(())
    }
}

#[derive(StructOpt)]
#[structopt(name = "chip8 decompiler", about = "a CHIP8 instruction deassembler.")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}
fn main() {
    let opts = Opt::from_args();

    let mut br = BufReader::new(File::open(opts.input).unwrap());
    let mut buffer = Vec::new();
    br.read_to_end(&mut buffer).unwrap();

    let prog = Program::try_from(buffer.as_slice()).expect("Bad program");
    println!("{}", prog);
}
