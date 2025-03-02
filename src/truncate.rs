use unicode_width::UnicodeWidthStr;

pub fn with_ellipses(text: &str, len: usize) -> String {
    if UnicodeWidthStr::width(text) <= len {
        return text.to_string();
    }

    let ellipses = "...";

    let mut result = String::new();
    let mut current_width = 0;

    for c in text.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width + ellipses.len() > len {
            break;
        }

        result.push(c);
        current_width += char_width;
    }

    result + ellipses
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
