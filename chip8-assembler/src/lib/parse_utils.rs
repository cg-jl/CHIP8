use std::collections::HashMap;
pub fn pexpr<T>(input: &str) -> Option<Value<T>> {
    let mut end_offt = input.len();
    if input.is_empty() {
        return None;
    }
    if let Some(comma) = input.find(',') {
        end_offt = comma;
    }
    Some(Value::Partial(&input[..end_offt]))
}
pub fn whitespace1(input: &str) -> Option<&str> {
    let c = input.chars().next().filter(|c| c.is_whitespace())?;
    Some(whitespace(&input[c.len_utf8()..]))
}
pub fn pcomma(input: &str) -> Option<&str> {
    whitespace(input).strip_prefix(",").map(whitespace)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value<'a, T> {
    Complete(T),
    Partial(&'a str),
}

fn parse_hex(a: &str) -> Option<(&str, u16)> {
    let a = a.strip_prefix("0x").or_else(|| a.strip_prefix("0X"))?;
    // at least one hex character
    let c = a.chars().next().filter(|x| x.is_ascii_hexdigit())?;
    let mut value: u16 = match c.to_ascii_lowercase() {
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
    };

    let mut offset = 1;
    for c in a[1..].chars() {
        if !c.is_ascii_hexdigit() {
            break;
        }
        let (next_val, overshoots) = value.overflowing_shl(4);
        if overshoots {
            return None; // too big.
        }
        value = next_val
            | match c.to_ascii_lowercase() {
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
            };
        offset += c.len_utf8();
    }
    Some((&a[offset..], value))
}

fn parse_dec(input: &str) -> Option<(&str, u16)> {
    let c = input.chars().next().filter(|x| x.is_ascii_digit())?;
    let mut value: u16 = match c.to_ascii_lowercase() {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        _ => unreachable!(),
    };

    let mut offset = 1;
    for c in input[1..].chars() {
        if !c.is_ascii_digit() {
            break;
        }
        let (next_val, overshoots) = value.overflowing_mul(10);
        if overshoots {
            return None;
        }
        value = next_val
            + match c.to_ascii_lowercase() {
                '0' => 0,
                '1' => 1,
                '2' => 2,
                '3' => 3,
                '4' => 4,
                '5' => 5,
                '6' => 6,
                '7' => 7,
                '8' => 8,
                '9' => 9,
                _ => unreachable!(),
            };
        offset += c.len_utf8();
    }
    Some((&input[offset..], value))
}

pub fn parse_num(input: &str) -> Option<(&str, u16)> {
    if let Some(v) = parse_hex(input) {
        Some(v)
    } else {
        parse_dec(input)
    }
}

pub fn parse_name(input: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    for c in input.chars() {
        if c != '@' && c != '_' && !c.is_alphabetic() {
            break;
        }
        offset += c.len_utf8();
    }
    if offset == 0 {
        return None;
    }
    for c in input.chars().skip(offset) {
        if c != '@' && c != '_' && !c.is_alphanumeric() {
            break;
        }
        offset += c.len_utf8();
    }

    Some((&input[offset..], &input[..offset]))
}

pub fn parse_const<'a>(
    input: &'a str,
    table: &HashMap<&str, Value<u16>>,
) -> Option<(&'a str, u16)> {
    // if either theres no more input (last value) or there's a whitespace after the dot.
    if let Some(input) = input
        .strip_prefix(".")
        .filter(|i| matches!(i.chars().next().filter(|c| c.is_whitespace()), Some(_)))
    {
        let current_address = table["."].consume(table)?;
        return Some((input, current_address));
    }
    if let Some((rest, name)) = parse_name(input) {
        let v = table.get(name)?.consume(table)?;
        Some((rest, v))
    } else {
        parse_num(input)
    }
}
impl<'a> Value<'a, u16> {
    pub fn consume(&self, table: &HashMap<&str, Value<u16>>) -> Option<u16> {
        match self {
            Value::Complete(t) => Some(*t),
            Value::Partial(input) => {
                // first term
                let (mut input, mut value) =
                    parse_const(input, table).map(|(a, b)| (whitespace(a), b))?;
                loop {
                    // this is actually overwritten in this statement.
                    #[allow(unused_assignments)]
                    let mut do_negate = false;
                    if let Some(c) = input.chars().next().filter(|c| c == &'-' || c == &'+') {
                        do_negate = c == '-';
                    } else {
                        break;
                    }

                    input = whitespace(&input[1..]);

                    let (rest, mut next_term) =
                        parse_const(input, table).map(|(a, b)| (whitespace(a), b))?;

                    if do_negate {
                        // safety: an add to a value with a 1 in the first
                        // bit will signify a substraction.
                        next_term = !next_term + 1;
                    }
                    input = rest;
                    value = value.wrapping_add(next_term);
                }
                Some(value)
            }
        }
    }
}

impl<'a> Value<'a, u8> {
    pub fn consume(&self, table: &HashMap<&str, Value<u16>>) -> Option<u8> {
        match self {
            Value::Complete(t) => Some(*t),
            Value::Partial(v) => Value::<u16>::Partial(v).consume(table).map(|x| x as u8),
        }
    }
}

impl<'a, T> From<T> for Value<'a, T> {
    fn from(v: T) -> Self {
        Value::Complete(v)
    }
}

pub fn whitespace(a: &str) -> &str {
    let mut offset = 0;
    for (i, c) in a.char_indices() {
        offset = i;
        if !c.is_whitespace() {
            break;
        }
    }
    &a[offset..]
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn whitespace() {
        assert_eq!(super::whitespace(" hello world!"), "hello world!");
        assert_eq!(super::whitespace1("hello, world!"), None);
        assert_eq!(
            super::whitespace1("\t   hello, world!"),
            Some("hello, world!")
        );
    }

    #[test]
    fn constants() {
        assert_eq!(parse_hex("0xf0"), Some(("", 0xf0)));
        assert_eq!(parse_hex("f0f"), None);
        assert_eq!(
            parse_hex("0xf0, hello, world!"),
            Some((", hello, world!", 0xf0))
        );
        assert_eq!(parse_dec("100"), Some(("", 100)));
        assert_eq!(parse_dec(""), None);
        assert_eq!(parse_dec("100 bytes"), Some((" bytes", 100)));

        let mut map = HashMap::<_, Value<u16>>::new();
        map.insert(".", 10.into());
        map.insert("hey", 25.into());
        assert_eq!(
            pexpr::<u16>(". + 10 - 3 + hey").and_then(|x| x.consume(&map)),
            Some(42u16)
        );
    }
}
