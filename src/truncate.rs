//! String truncation tools.

use unicode_width::UnicodeWidthStr;

pub fn with_ellipses(text: &str, len: usize) -> String {
    if UnicodeWidthStr::width(text) <= len {
        return String::from(text);
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

    // Truncate ellipses if necessary
    result + &ellipses[0..len.min(ellipses.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal() {
        assert_eq!(with_ellipses("hello", 5), "hello");
    }

    #[test]
    fn larger() {
        assert_eq!(with_ellipses("hello", 6), "hello");
    }

    #[test]
    fn shorter() {
        assert_eq!(with_ellipses("hello", 4), "h...");
    }

    #[test]
    fn too_short() {
        assert_eq!(with_ellipses("hello", 3), "...");
    }

    #[test]
    fn much_too_short() {
        assert_eq!(with_ellipses("hello", 2), "..");
    }

    #[test]
    fn empty() {
        assert_eq!(with_ellipses("hello", 0), "");
    }
}
