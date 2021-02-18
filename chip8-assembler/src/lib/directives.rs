use crate::parse_utils::*;
pub fn repeat(input: &str) -> Option<(u8, u16)> {
    if !input.starts_with(".repeat") {
        return None;
    }
    let (input, x) = parse_num(whitespace1(&input[7..])?)?;
    let input = pcomma(input)?;
    let (_, y) = parse_num(input)?;
    Some((x as u8, y))
}

pub fn reserve(input: &str) -> Option<u16> {
    if !input.starts_with(".reserve") {
        return None;
    }
    let (_, x) = parse_num(whitespace1(&input[8..])?)?;
    Some(x)
}

pub fn entrypoint(input: &str) -> Option<&str> {
    if !input.starts_with(".entrypoint") {
        return None;
    }
    let (_, inp) = parse_name(whitespace1(&input[11..])?)?;
    Some(inp)
}

pub fn sequence_bytes(input: &str) -> Option<Vec<u8>> {
    if !input.starts_with("db") {
        return None;
    }

    let mut values = Vec::new();
    let (mut input, first_value) = parse_num(whitespace1(&input[2..])?)?;
    values.push(first_value as u8);

    loop {
        if let Some(next_input) = pcomma(whitespace(input)).map(whitespace) {
            // new value
            let (next_input, next_value) = parse_num(next_input)?;
            input = next_input;
            values.push(next_value as u8);
            continue;
        }
        break;
    }

    Some(values)
}

#[cfg(test)]
mod tests {
    #[test]
    fn repeat() {
        assert_eq!(super::repeat(".repeat 0x80, 15"), Some((0x80, 15)));
    }
}
