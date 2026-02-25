use std::fmt;
use std::fmt::{Display, Formatter};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct PostTitle(String);

impl PostTitle {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid title: cannot be empty.".to_string());
        }

        let grapheme_count = trimmed.graphemes(true).count();

        if grapheme_count > 100 {
            return Err("Invalid title: cannot be longer than 100 characters.".to_string());
        }

        // Check if title contains only digits
        let has_non_numeric = trimmed
            .chars()
            .any(|c| !c.is_numeric() && !c.is_whitespace());
        if !has_non_numeric {
            return Err("Invalid title: cannot contain only numbers.".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for PostTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for PostTitle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::PostTitle;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    // Example-based tests
    #[test]
    fn empty_title_is_rejected() {
        let result = PostTitle::parse("".into());
        assert_err!(result);
    }

    #[test]
    fn long_title_is_rejected() {
        let long_title = "a".repeat(101);
        let result = PostTitle::parse(long_title);
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_is_rejected() {
        let result = PostTitle::parse("12345".into());
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_and_spaces_is_rejected() {
        let result = PostTitle::parse("123 456".into());
        assert_err!(result);
    }

    #[test]
    fn title_with_numbers_and_letters_is_accepted() {
        let result = PostTitle::parse("Post123".into());
        assert_ok!(result);
    }

    #[test]
    fn title_with_letters_and_numbers_is_accepted() {
        let result = PostTitle::parse("123Post".into());
        assert_ok!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn valid_titles_with_valid_length_are_accepted(
            title in r"[a-zA-Z][a-zA-Z0-9 ]{0,99}",
        ) {
            let result = PostTitle::parse(title);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn titles_longer_than_100_chars_are_rejected(
            title in r"[a-zA-Z0-9]{101,150}",
        ) {
            let result = PostTitle::parse(title);
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_titles_are_rejected(
            title in r"\s{1,50}",
        ) {
            let result = PostTitle::parse(title);
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_are_rejected(
            title in r"[0-9]{1,50}",
        ) {
            let result = PostTitle::parse(title);
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_with_spaces_are_rejected(
            num1 in r"[0-9]{1,20}",
            num2 in r"[0-9]{1,20}",
        ) {
            let title = format!("{} {}", num1, num2);
            let result = PostTitle::parse(title);
            prop_assert!(result.is_err());
        }
    }
}
