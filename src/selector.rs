use anyhow::{Result, bail};

pub fn parse(s: &str, len: usize) -> Result<usize> {
    if len == 0 {
        bail!("no versions available");
    }
    if s == "current" {
        return Ok(len - 1);
    }
    match s.parse::<usize>() {
        Ok(index) if index < len => Ok(index),
        Ok(index) => bail!("version {index} is out of range (valid: 0..={})", len - 1),
        Err(_) => bail!("invalid version '{s}': expected an index or 'current'"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn current_selects_last() {
        assert_eq!(parse("current", 3).unwrap(), 2);
    }

    #[test]
    fn integer_within_range_selects_index() {
        assert_eq!(parse("0", 3).unwrap(), 0);
        assert_eq!(parse("2", 3).unwrap(), 2);
    }

    #[test]
    fn out_of_range_is_an_error() {
        let err = parse("3", 3).unwrap_err().to_string();
        assert!(err.contains("out of range"), "{err}");
        assert!(err.contains("0..=2"), "{err}");
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(parse("latest", 3).is_err());
        assert!(parse("-1", 3).is_err());
        assert!(parse("", 3).is_err());
    }

    #[test]
    fn empty_list_rejects_everything() {
        assert!(parse("current", 0).is_err());
        assert!(parse("0", 0).is_err());
    }
}
