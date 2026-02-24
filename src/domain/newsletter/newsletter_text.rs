use std::fmt;
use std::fmt::{Display, Formatter};
#[derive(Debug)]
pub struct NewsletterText(String);

impl NewsletterText {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid newsletter text: cannot be empty.".to_string());
        }

        if trimmed.len() > 50_000 {
            return Err(
                "Invalid newsletter text: cannot be longer than 50,000 characters.".to_string(),
            );
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for NewsletterText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for NewsletterText {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::NewsletterText;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    // Example-based tests for Newsletter Text
    #[test]
    fn empty_text_is_rejected() {
        let result = NewsletterText::parse("".into());
        assert_err!(result);
    }

    #[test]
    fn whitespace_only_text_is_rejected() {
        let result = NewsletterText::parse("   \n\t   ".into());
        assert_err!(result);
    }

    #[test]
    fn text_exceeding_max_length_is_rejected() {
        let long_text = "a".repeat(50_001);
        let result = NewsletterText::parse(long_text);
        assert_err!(result);
    }

    #[test]
    fn valid_text_is_accepted() {
        let result =
            NewsletterText::parse("This is the plain text version of the newsletter.".into());
        assert_ok!(result);
    }

    #[test]
    fn text_at_max_length_is_accepted() {
        let text = "a".repeat(50_000);
        let result = NewsletterText::parse(text);
        assert_ok!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn text_content_within_limits_is_accepted(
            content in r"[a-zA-Z0-9 .!?,]{10,1000}",
        ) {
            let result = NewsletterText::parse(content);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn whitespace_only_text_content_is_rejected(
            text in r"\s{1,100}",
        ) {
            let result = NewsletterText::parse(text);
            prop_assert!(result.is_err());
        }

        #[test]
        fn very_long_text_exceeding_limit_is_rejected(
            // Generate content that will exceed 50,000 chars
            size in 50_001..60_000_usize,
        ) {
            let text = "a".repeat(size);
            let result = NewsletterText::parse(text);
            prop_assert!(result.is_err());
        }
    }
}
