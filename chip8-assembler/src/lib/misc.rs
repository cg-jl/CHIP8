use crate::parse_utils::*;
pub fn constant(input: &str) -> Option<(&str, Value<u16>)> {
    let (mut input, name) = parse_name(&input).map(|(a, b)| (whitespace(a), b))?;
    input = input.strip_prefix("=").map(whitespace)?;
    let value = pexpr(&input)?;
    Some((name, value))
}

pub fn label(input: &str) -> Option<&str> {
    let (input, name) = parse_name(input)?;
    let c = input.chars().next()?;
    if c != ':' {
        return None;
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant() {
        assert_eq!(
            super::constant("hey = 10"),
            Some(("hey", Value::Partial("10")))
        );
    }

    #[test]
    fn label() {
        assert_eq!(super::label("hello:"), Some("hello"));
    }
}
