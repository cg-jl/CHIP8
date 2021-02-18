use crate::parse_utils::*;
use std::collections::HashMap;
#[derive(Debug, PartialEq, Eq)]
pub enum Argument<'a> {
    Constant(Value<'a, u8>),
    Register(Value<'a, u8>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction<'a> {
    Load {
        register: u8, // compulsory register
        value: Argument<'a>,
    },
    Add {
        target: u8,          // valid register
        value: Argument<'a>, // either constant or register
    },
    Sub {
        target: u8, // valid register
        value: u8,  // no constant, valid register
        inverse: bool,
    },
    Shift {
        from: u8,
        target: u8,
        is_left: bool,
    },
    And {
        // both valid registers
        from: u8,
        target: u8,
    },
    Or {
        // both valid registers
        from: u8,
        target: u8,
    },
    Xor {
        from: u8,
        target: u8,
    },
    Jump {
        uses_zero: bool, // jp0
        target: Value<'a, u16>,
    },
    Call(Value<'a, u16>),
    Return,
    ConditionalSkip {
        a: u8,
        b: Argument<'a>,
        negated: bool,
    },
    Dump(u8),
    LoadR(u8),
    LoadI(Value<'a, u16>),
    Font(u8),
    AddI(u8), // register
    LoadDelay(u8),
    SetDelay(u8),
    SetSound(u8),
    BinaryCodedDecimal(u8),
    Clear,
    Draw {
        x: u8,
        y: u8,
        height: Value<'a, u8>,
    },
    LoadKey(u8),
    ConditionalKey {
        register: u8,
        negated: bool,
    },
    Random {
        target: u8,
        mask: Value<'a, u8>,
    },
}

impl<'a> Instruction<'a> {
    pub fn compile(&self, table: &HashMap<&str, Value<u16>>) -> Option<u16> {
        let v = match self {
            Instruction::Load { register, value } => match value {
                Argument::Constant(x) => {
                    let nn = x.consume(&table)?;
                    0x6000 | (*register as u16) << 8 | (nn as u16)
                }
                Argument::Register(x) => {
                    let vx = x.consume(&table)? & 0xf;
                    0x8000 | (*register as u16) << 8 | (vx as u16) << 4
                }
            },
            Instruction::Add { target, value } => match value {
                Argument::Constant(x) => {
                    let nn = x.consume(&table)?;
                    0x7000 | (*target as u16) << 8 | (nn as u16)
                }
                Argument::Register(r) => {
                    let vy = r.consume(&table)?;
                    0x8004 | (*target as u16) << 8 | (vy as u16) << 4
                }
            },
            Instruction::Sub {
                target: vx,
                value: vy,
                inverse,
            } => (if *inverse { 0x8005 } else { 0x8007 }) | (*vx as u16) << 8 | (*vy as u16) << 4,
            Instruction::Shift {
                target: vy,
                from: vx,
                is_left,
            } => 0x8000 | (*vx as u16) << 8 | (*vy as u16) << 4 | if !*is_left { 6 } else { 0xe },
            Instruction::And {
                from: vx,
                target: vy,
            } => 0x8002 | (*vx as u16) << 8 | (*vy as u16) << 4,
            Instruction::Or {
                from: vx,
                target: vy,
            } => 0x8001 | (*vx as u16) << 8 | (*vy as u16) << 4,
            Instruction::Xor {
                from: vx,
                target: vy,
            } => 0x8003 | (*vx as u16) << 8 | (*vy as u16) << 4,
            Instruction::Jump { uses_zero, target } => {
                let target = target.consume(table)? & 0xfff;
                (if *uses_zero { 0xb000 } else { 0x1000 }) | target
            }
            Instruction::Call(target) => {
                let target = target.consume(table)? & 0xfff;
                0x2000 | target
            }
            Instruction::Return => 0xee,
            Instruction::ConditionalSkip { a: vx, b, negated } => {
                let code = match b {
                    Argument::Register(vy) => {
                        let vy = vy.consume(table)?;
                        (if *negated { 0x9000 } else { 0x5000 }) | (vy as u16) << 4
                    }
                    Argument::Constant(nn) => {
                        let nn = nn.consume(table)?;
                        (if *negated { 0x4000 } else { 0x3000 }) | (nn as u16)
                    }
                };

                code | (*vx as u16) << 8
            }
            Instruction::Dump(vx) => 0xf055 | (*vx as u16) << 8,
            Instruction::LoadR(vx) => 0xf065 | (*vx as u16) << 8,
            Instruction::LoadI(v) => {
                let v = v.consume(table)?;
                0xa000 | v
            }
            Instruction::Font(vx) => 0xf029 | (*vx as u16) << 8,
            Instruction::AddI(vx) => 0xf01e | (*vx as u16) << 8,
            Instruction::LoadDelay(vx) => 0xf007 | (*vx as u16) << 8,
            Instruction::SetDelay(vx) => 0xf015 | (*vx as u16) << 8,
            Instruction::SetSound(vx) => 0xf017 | (*vx as u16) << 8,
            Instruction::BinaryCodedDecimal(vx) => 0xf033 | (*vx as u16) << 8,
            Instruction::Clear => 0xe0,
            Instruction::Draw {
                x: vx,
                y: vy,
                height,
            } => {
                let height = height.consume(table)? & 0xf;
                0xd000 | (*vx as u16) << 8 | (*vy as u16) << 4 | height as u16
            }
            Instruction::LoadKey(vx) => 0xf00a | (*vx as u16) << 8,
            Instruction::ConditionalKey {
                negated,
                register: vx,
            } => 0xe000 | (*vx as u16) << 8 | if *negated { 0xa1 } else { 0x9e },
            Instruction::Random { target: vx, mask } => {
                let mask = mask.consume(table)? as u16;
                0xc00 | (*vx as u16) << 8 | mask & 0xff
            }
        };
        Some(v)
    }
}

fn parg(input: &str) -> Option<Argument> {
    if let Some(x) = preg(input) {
        Some(Argument::Register(Value::Complete(x)))
    } else {
        let expr = pexpr(input)?;
        Some(Argument::Constant(expr))
    }
}
fn preg(input: &str) -> Option<u8> {
    if let Some('V') = input.chars().next() {
        if let Some(c) = input[1..].chars().next() {
            if c.is_ascii_hexdigit() {
                return Some(match c.to_ascii_lowercase() {
                    '0' => 0x0,
                    '1' => 0x1,
                    '2' => 0x2,
                    '3' => 0x3,
                    '4' => 0x4,
                    '5' => 0x5,
                    '6' => 0x6,
                    '7' => 0x7,
                    '8' => 0x8,
                    '9' => 0x9,
                    'a' => 0xa,
                    'b' => 0xb,
                    'c' => 0xc,
                    'd' => 0xd,
                    'e' => 0xe,
                    'f' => 0xf,
                    _ => unreachable!(),
                });
            }
        }
    }
    None
}
fn load(mut input: &str) -> Option<Instruction> {
    if &input[..2] != "LD" {
        return None;
    }
    input = whitespace1(&input[2..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let value = parg(input)?;
    Some(Instruction::Load {
        register: vx,
        value,
    })
}
fn add(mut input: &str) -> Option<Instruction> {
    if &input[..3] != "ADD" {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let value = parg(input)?;
    Some(Instruction::Add { target: vx, value })
}
fn sub(mut input: &str) -> Option<Instruction> {
    let mut inverse = false;
    if input.starts_with("SBI") {
        inverse = true;
    } else if !input.starts_with("SUB") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let vy = preg(input)?;
    Some(Instruction::Sub {
        target: vx,
        value: vy,
        inverse,
    })
}
fn shift(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("SH") {
        return None;
    }
    input = &input[2..];
    let mut is_left = false;
    let c = input.chars().next()?;
    match c {
        'L' => {
            is_left = true;
        }
        'R' => {}
        _ => {
            return None;
        }
    }
    input = whitespace1(&input[1..])?;
    let vx = preg(input)?;
    let mut vy = vx;
    if let Some(input) = pcomma(&input[2..]) {
        vy = preg(input)?;
    }
    Some(Instruction::Shift {
        is_left,
        from: vx,
        target: vy,
    })
}
fn and(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("AND") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let vy = preg(input)?;
    Some(Instruction::And {
        from: vy,
        target: vx,
    })
}
fn xor(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("XOR") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let vy = preg(input)?;
    Some(Instruction::Xor {
        from: vy,
        target: vx,
    })
}
fn or(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("OR") {
        return None;
    }
    input = whitespace1(&input[2..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let vy = preg(input)?;
    Some(Instruction::Or {
        from: vy,
        target: vx,
    })
}

fn jmp(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("JP") {
        return None;
    }
    input = &input[2..];
    let mut uses_zero = false;
    let c = input.chars().next()?;
    if c == '0' {
        uses_zero = true;
        input = &input[1..];
    }
    input = whitespace1(input)?;
    let addr = pexpr(input)?;

    Some(Instruction::Jump {
        target: addr,
        uses_zero,
    })
}

fn call(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("CALL") {
        return None;
    }
    input = whitespace1(&input[4..])?;
    let addr = pexpr(input)?;
    Some(Instruction::Call(addr))
}
fn ret(input: &str) -> Option<Instruction> {
    if !input.starts_with("RET") {
        None
    } else {
        Some(Instruction::Return)
    }
}
fn conditional_skip(mut input: &str) -> Option<Instruction> {
    let mut negated = false;
    if input.starts_with("SNE") {
        negated = true;
    } else if !input.starts_with("SEQ") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let b = parg(input)?;
    Some(Instruction::ConditionalSkip { a: vx, b, negated })
}

fn dump(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("DMP") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::Dump(vx))
}

fn load_registers(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("LDR") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::LoadR(vx))
}
fn set_address(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("LDI") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let addr = pexpr(input)?;
    Some(Instruction::LoadI(addr))
}
fn font(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("FNT") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::Font(vx))
}
fn add_i(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("ADDI") {
        return None;
    }
    input = whitespace1(&input[4..])?;
    let vx = preg(input)?;
    Some(Instruction::AddI(vx))
}
fn load_delay(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("LDD") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::LoadDelay(vx))
}
fn set_delay(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("DLY") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::SetDelay(vx))
}
fn set_sound(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("SND") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::SetSound(vx))
}
fn bcd(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("BCD") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::BinaryCodedDecimal(vx))
}
fn clear(input: &str) -> Option<Instruction> {
    if input != "CLR" {
        return None;
    }
    Some(Instruction::Clear)
}
fn draw(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("DRW") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    input = pcomma(&input[2..])?;
    let vy = preg(input)?;
    input = pcomma(&input[2..])?;
    let height = pexpr(input)?;
    Some(Instruction::Draw {
        x: vx,
        y: vy,
        height,
    })
}
fn load_key(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("LDK") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::LoadKey(vx))
}
fn conditional_key(mut input: &str) -> Option<Instruction> {
    let mut negated = false;
    if input.starts_with("SNK") {
        negated = true;
    } else if !input.starts_with("SIK") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    Some(Instruction::ConditionalKey {
        register: vx,
        negated,
    })
}
fn random(mut input: &str) -> Option<Instruction> {
    if !input.starts_with("RND") {
        return None;
    }
    input = whitespace1(&input[3..])?;
    let vx = preg(input)?;
    let mut mask = Value::Complete(0xff);
    if let Some(input) = pcomma(&input[2..]) {
        mask = pexpr(input)?;
    }
    Some(Instruction::Random { target: vx, mask })
}
pub fn any(input: &str) -> Option<Instruction> {
    const PARSERS: &[fn(&str) -> Option<Instruction>] = &[
        clear,
        ret,
        random,
        conditional_key,
        conditional_skip,
        and,
        or,
        xor,
        load_key,
        load_registers,
        load_delay,
        jmp,
        call,
        dump,
        bcd,
        add_i,
        add,
        sub,
        set_address,
        set_delay,
        set_sound,
        font,
        shift,
        load,
        draw,
    ];

    for p in PARSERS {
        if let Some(i) = p(input) {
            return Some(i);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    mod parsing {
        use super::*;

        #[test]
        fn draw() {
            assert_eq!(
                super::draw("DRW V0, V1, sprite_length"),
                Some(Instruction::Draw {
                    x: 0,
                    y: 1,
                    height: Value::Partial("sprite_length")
                })
            );
        }

        #[test]
        fn load() {
            assert_eq!(
                super::load("LD V0, VE"),
                Some(Instruction::Load {
                    register: 0,
                    value: Argument::Register(Value::Complete(0xe))
                })
            );
            assert_eq!(
                super::load("LD V0, 3"),
                Some(Instruction::Load {
                    register: 0,
                    value: Argument::Constant(Value::Partial("3"))
                })
            );
        }

        #[test]
        fn shift() {
            assert_eq!(
                super::shift("SHR V0, VE"),
                Some(Instruction::Shift {
                    is_left: false,
                    target: 0xe,
                    from: 0
                })
            );
            assert_eq!(
                super::shift("SHL V0, VE"),
                Some(Instruction::Shift {
                    is_left: true,
                    target: 0xe,
                    from: 0
                })
            );
            assert_eq!(
                super::shift("SHL V0"),
                Some(Instruction::Shift {
                    is_left: true,
                    target: 0,
                    from: 0
                })
            );
            assert_eq!(
                super::shift("SHR V0"),
                Some(Instruction::Shift {
                    is_left: false,
                    target: 0,
                    from: 0
                })
            );
        }

        #[test]
        fn font() {
            assert_eq!(super::font("FNT V0"), Some(Instruction::Font(0)));
        }

        #[test]
        fn set_sound() {
            assert_eq!(super::set_sound("SND V0"), Some(Instruction::SetSound(0)));
        }

        #[test]
        fn set_delay() {
            assert_eq!(super::set_delay("DLY V0"), Some(Instruction::SetDelay(0)));
        }

        #[test]
        fn set_address() {
            assert_eq!(
                super::set_address("LDI 0x202"),
                Some(Instruction::LoadI(Value::Partial("0x202")))
            );
        }

        #[test]
        fn sub() {
            assert_eq!(
                super::sub("SUB V0, VE"),
                Some(Instruction::Sub {
                    target: 0,
                    value: 0xe,
                    inverse: false
                })
            );
            assert_eq!(
                super::sub("SBI V0, VE"),
                Some(Instruction::Sub {
                    target: 0,
                    value: 0xe,
                    inverse: true
                })
            );
        }

        #[test]
        fn add() {
            assert_eq!(
                super::add("ADD V0, VE"),
                Some(Instruction::Add {
                    target: 0,
                    value: Argument::Register(Value::Complete(0xe))
                })
            );
            assert_eq!(
                super::add("ADD V0, 0xff"),
                Some(Instruction::Add {
                    target: 0,
                    value: Argument::Constant(Value::Partial("0xff"))
                })
            );
        }

        #[test]
        fn add_i() {
            assert_eq!(super::add_i("ADDI V0"), Some(Instruction::AddI(0)));
        }

        #[test]
        fn bcd() {
            assert_eq!(
                super::bcd("BCD V0"),
                Some(Instruction::BinaryCodedDecimal(0))
            );
        }

        #[test]
        fn dump() {
            assert_eq!(super::dump("DMP V0"), Some(Instruction::Dump(0)));
        }

        #[test]
        fn call() {
            assert_eq!(
                super::call("CALL draw_number"),
                Some(Instruction::Call(Value::Partial("draw_number")))
            );
        }

        #[test]
        fn jmp() {
            assert_eq!(
                super::jmp("JP0 0x202"),
                Some(Instruction::Jump {
                    target: Value::Partial("0x202"),
                    uses_zero: true,
                })
            );
            assert_eq!(
                super::jmp("JP 0x202"),
                Some(Instruction::Jump {
                    target: Value::Partial("0x202"),
                    uses_zero: false,
                })
            );
        }

        #[test]
        fn load_delay() {
            assert_eq!(super::load_delay("LDD V0"), Some(Instruction::LoadDelay(0)));
        }

        #[test]
        fn load_registers() {
            assert_eq!(super::load_registers("LDR V0"), Some(Instruction::LoadR(0)));
        }

        #[test]
        fn load_key() {
            assert_eq!(super::load_key("LDK V0"), Some(Instruction::LoadKey(0)));
        }

        #[test]
        fn xor() {
            assert_eq!(
                super::xor("XOR V0, VF"),
                Some(Instruction::Xor {
                    from: 0xf,
                    target: 0
                })
            );
        }

        #[test]
        fn or() {
            assert_eq!(
                super::or("OR V0, VF"),
                Some(Instruction::Or {
                    from: 0xf,
                    target: 0
                })
            );
        }

        #[test]
        fn and() {
            assert_eq!(
                super::and("AND V0, VF"),
                Some(Instruction::And {
                    from: 0xf,
                    target: 0
                })
            );
        }
    }
}
