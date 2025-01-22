pub fn with_ellipses(text: &str, len: usize) -> String {
    if text.len() <= len {
        return text.to_string();
    }

    let left = len.saturating_sub(3);

    let truncated = text
        .char_indices()
        .take_while(|(i, _)| *i < left)
        .map(|(_, c)| c)
        .collect::<String>();

    truncated + &".".repeat(std::cmp::min(len, 3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_test_equal() {
        assert_eq!(with_ellipses("hello", 5), "hello");
    }

    #[test]
    fn truncate_test_larger() {
        assert_eq!(with_ellipses("hello", 6), "hello");
    }

    #[test]
    fn truncate_test_shorter() {
        assert_eq!(with_ellipses("hello", 4), "h...");
    }

    #[test]
    fn truncate_test_too_short() {
        assert_eq!(with_ellipses("hello", 3), "...");
    }

    #[test]
    fn truncate_test_much_too_short() {
        assert_eq!(with_ellipses("hello", 2), "..");
    }

    #[test]
    fn truncate_test_empty() {
        assert_eq!(with_ellipses("hello", 0), "");
    }
}
