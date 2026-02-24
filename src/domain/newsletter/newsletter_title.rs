use std::fmt;
use std::fmt::{Display, Formatter};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct NewsletterTitle(String);

impl NewsletterTitle {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid newsletter title: cannot be empty.".to_string());
        }

        let grapheme_count = trimmed.graphemes(true).count();

        if grapheme_count > 200 {
            return Err(
                "Invalid newsletter title: cannot be longer than 200 characters.".to_string(),
            );
        }

        // Check if title contains only digits
        let has_non_numeric = trimmed
            .chars()
            .any(|c| !c.is_numeric() && !c.is_whitespace());
        if !has_non_numeric {
            return Err("Invalid newsletter title: cannot contain only numbers.".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for NewsletterTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for NewsletterTitle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::NewsletterTitle;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    // Example-based tests for Newsletter Title
    #[test]
    fn empty_title_is_rejected() {
        let result = NewsletterTitle::parse("".into());
        assert_err!(result);
    }

    #[test]
    fn long_title_is_rejected() {
        let long_title = "a".repeat(201);
        let result = NewsletterTitle::parse(long_title);
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_is_rejected() {
        let result = NewsletterTitle::parse("12345".into());
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_and_spaces_is_rejected() {
        let result = NewsletterTitle::parse("123 456".into());
        assert_err!(result);
    }

    #[test]
    fn title_with_numbers_and_letters_is_accepted() {
        let result = NewsletterTitle::parse("Newsletter123".into());
        assert_ok!(result);
    }

    #[test]
    fn title_with_letters_and_numbers_is_accepted() {
        let result = NewsletterTitle::parse("123Newsletter".into());
        assert_ok!(result);
    }

    #[test]
    fn title_at_max_length_is_accepted() {
        let title = "a".repeat(200);
        let result = NewsletterTitle::parse(title);
        assert_ok!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn valid_titles_with_valid_length_are_accepted(
            title in r"[a-zA-Z][a-zA-Z0-9 ]{0,199}",
        ) {
            let result = NewsletterTitle::parse(title);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn titles_longer_than_200_chars_are_rejected(
            title in r"[a-zA-Z0-9]{201,250}",
        ) {
            let result = NewsletterTitle::parse(title);
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_titles_are_rejected(
            title in r"\s{1,50}",
        ) {
            let result = NewsletterTitle::parse(title);
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_are_rejected(
            title in r"[0-9]{1,50}",
        ) {
            let result = NewsletterTitle::parse(title);
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_with_spaces_are_rejected(
            num1 in r"[0-9]{1,20}",
            num2 in r"[0-9]{1,20}",
        ) {
            let title = format!("{} {}", num1, num2);
            let result = NewsletterTitle::parse(title);
            prop_assert!(result.is_err());
        }

        #[test]
        fn titles_with_mixed_alphanumeric_are_accepted(
            prefix in r"[a-zA-Z]{1,10}",
            number in r"[0-9]{1,5}",
            suffix in r"[a-zA-Z ]{0,20}",
        ) {
            let title = format!("{}{}{}", prefix, number, suffix);
            let result = NewsletterTitle::parse(title);
            prop_assert!(result.is_ok());
        }
    }
}
