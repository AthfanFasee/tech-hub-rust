use std::fmt::{self, Display, Formatter};

use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct CommentText(String);

impl CommentText {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid comment: cannot be empty.".to_string());
        }

        let grapheme_count = trimmed.graphemes(true).count();

        if grapheme_count > 200 {
            return Err("Invalid comment: cannot exceed 200 characters.".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for CommentText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for CommentText {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;
    use unicode_segmentation::UnicodeSegmentation;

    use super::CommentText;

    // Example-based tests
    #[test]
    fn a_comment_with_200_chars_is_valid() {
        let comment = "a".repeat(200);
        assert_ok!(CommentText::parse(comment));
    }

    #[test]
    fn a_comment_longer_than_200_chars_is_rejected() {
        let comment = "a".repeat(201);
        assert_err!(CommentText::parse(comment));
    }

    #[test]
    fn empty_string_is_rejected() {
        let comment = "".to_string();
        assert_err!(CommentText::parse(comment));
    }

    #[test]
    fn comment_with_emojis_counts_graphemes_correctly() {
        // 5 emojis = 5 graphemes (should be valid)
        let comment = "👍😀🎉❤️🔥".to_string();
        assert_ok!(CommentText::parse(comment));
    }

    #[test]
    fn comment_with_special_characters_is_valid() {
        let comment = "Great post! @user #awesome https://example.com".to_string();
        assert_ok!(CommentText::parse(comment));
    }

    // Property-based tests
    proptest! {
        #[test]
        fn comments_with_valid_length_are_accepted(
            // Start with non-space, then allow spaces
            comment in r"[a-zA-Z0-9][a-zA-Z0-9 ]{0,199}",
        ) {
            let result = CommentText::parse(comment);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn comments_longer_than_200_chars_are_rejected(
            comment in r"[a-zA-Z0-9]{201,300}",
        ) {
            let result = CommentText::parse(comment);
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_comments_are_rejected(
            comment in r"\s{1,50}",
        ) {
            let result = CommentText::parse(comment);
            prop_assert!(result.is_err());
        }

        #[test]
        fn comments_with_unicode_in_valid_range_are_handled_correctly(
            comment in prop::collection::vec(any::<char>(), 1..=200)
                .prop_map(|chars| chars.into_iter().collect::<String>())
        ) {
            let result = CommentText::parse(comment.clone());
            let trimmed = comment.trim();
            let grapheme_count = trimmed.graphemes(true).count();

            // Property: Comment is valid if not empty and <= 200 graphemes
            if !trimmed.is_empty() && grapheme_count <= 200 {
                prop_assert!(result.is_ok(), "Expected Ok but got {:?} for comment: {:?}", result, comment);
            } else {
                prop_assert!(result.is_err(), "Expected Err but got Ok for comment: {:?}", comment);
            }
        }
    }
}
