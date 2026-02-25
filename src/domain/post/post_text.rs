use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct PostText(String);

impl PostText {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid text: cannot be empty.".to_string());
        }

        if trimmed.len() > 10_000 {
            return Err("Invalid text: cannot be longer than 10,000 characters.".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for PostText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for PostText {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::PostText;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    // Example-based tests
    #[test]
    fn empty_text_is_rejected() {
        let result = PostText::parse("".into());
        assert_err!(result);
    }

    #[test]
    fn text_exceeding_max_length_is_rejected() {
        let long_text = "a".repeat(10_001);
        let result = PostText::parse(long_text);
        assert_err!(result);
    }

    #[test]
    fn valid_text_is_accepted() {
        let result = PostText::parse("This is a valid post text with proper content.".into());
        assert_ok!(result);
    }

    #[test]
    fn text_at_max_length_is_accepted() {
        let text = "a".repeat(10_000);
        let result = PostText::parse(text);
        assert_ok!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn whitespace_only_text_is_rejected(
            text in r"\s{1,50}",
        ) {
            let result = PostText::parse(text);
            prop_assert!(result.is_err());
        }

        #[test]
        fn text_content_within_limits_is_accepted(
            content in r"[a-zA-Z0-9 .!?,]{10,1000}",
        ) {
            let result = PostText::parse(content);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn very_long_text_exceeding_limit_is_rejected(
            size in 10_001..11_000_usize,
        ) {
            let text = "a".repeat(size);
            let result = PostText::parse(text);
            prop_assert!(result.is_err());
        }
    }
}
