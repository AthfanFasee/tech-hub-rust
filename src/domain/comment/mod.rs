mod comment_text;
mod types;

pub use types::*;

pub use comment_text::CommentText;
use uuid::Uuid;

#[derive(Debug)]
pub struct Comment {
    pub text: CommentText,
    pub post_id: Uuid,
}

impl Comment {
    pub fn new(text: String, post_id: String) -> Result<Self, String> {
        let post_id = Uuid::parse_str(&post_id)
            .map_err(|_| "Invalid post_id: must be a valid UUID".to_string())?;

        Ok(Self {
            text: CommentText::parse(text)?,
            post_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Comment;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;
    use unicode_segmentation::UnicodeSegmentation;
    use uuid::Uuid;

    // Example-based tests
    #[test]
    fn a_comment_with_200_chars_is_valid() {
        let comment = "a".repeat(200);
        let post_id = Uuid::new_v4().to_string();
        assert_ok!(Comment::new(comment, post_id));
    }

    #[test]
    fn a_comment_longer_than_200_chars_is_rejected() {
        let comment = "a".repeat(201);
        let post_id = Uuid::new_v4().to_string();
        assert_err!(Comment::new(comment, post_id));
    }

    #[test]
    fn empty_string_is_rejected() {
        let comment = "".to_string();
        let post_id = Uuid::new_v4().to_string();
        assert_err!(Comment::new(comment, post_id));
    }

    #[test]
    fn invalid_uuid_is_rejected() {
        let comment = "Valid comment text".to_string();
        let invalid_uuid = "not-a-uuid".to_string();
        assert_err!(Comment::new(comment, invalid_uuid));
    }

    #[test]
    fn valid_comment_with_valid_uuid_is_accepted() {
        let comment = "This is a great post!".to_string();
        let post_id = Uuid::new_v4().to_string();
        assert_ok!(Comment::new(comment, post_id));
    }

    #[test]
    fn comment_with_emojis_counts_graphemes_correctly() {
        // 5 emojis = 5 graphemes (should be valid)
        let comment = "👍😀🎉❤️🔥".to_string();
        let post_id = Uuid::new_v4().to_string();
        assert_ok!(Comment::new(comment, post_id));
    }

    #[test]
    fn comment_with_special_characters_is_valid() {
        let comment = "Great post! @user #awesome https://example.com".to_string();
        let post_id = Uuid::new_v4().to_string();
        assert_ok!(Comment::new(comment, post_id));
    }

    // Property-based tests
    proptest! {
        #[test]
        fn comments_with_valid_length_are_accepted(
            // Start with non-space, then allow spaces
            comment in r"[a-zA-Z0-9][a-zA-Z0-9 ]{0,199}",
        ) {
            let post_id = Uuid::new_v4().to_string();
            let result = Comment::new(comment, post_id);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn comments_longer_than_200_chars_are_rejected(
            comment in r"[a-zA-Z0-9]{201,300}",
        ) {
            let post_id = Uuid::new_v4().to_string();
            let result = Comment::new(comment, post_id);
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_comments_are_rejected(
            comment in r"\s{1,50}",
        ) {
            let post_id = Uuid::new_v4().to_string();
            let result = Comment::new(comment, post_id);
            prop_assert!(result.is_err());
        }

        #[test]
        fn invalid_uuid_strings_are_rejected(
            comment in r"[a-zA-Z0-9 ]{1,50}",
            invalid_uuid in r"[a-z]{1,20}",
        ) {
            // Generate strings that are NOT valid UUIDs
            let result = Comment::new(comment, invalid_uuid);
            prop_assert!(result.is_err());
        }

        #[test]
        fn valid_uuids_with_valid_comments_are_accepted(
            // Ensure comment starts with non-space
            comment in r"[a-zA-Z0-9][a-zA-Z0-9 .!?]{0,199}",
        ) {
            // Generate a valid UUID
            let post_id = Uuid::new_v4().to_string();
            let result = Comment::new(comment, post_id);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn comments_with_unicode_in_valid_range_are_handled_correctly(
            comment in prop::collection::vec(any::<char>(), 1..=200)
                .prop_map(|chars| chars.into_iter().collect::<String>())
        ) {
            let post_id = Uuid::new_v4().to_string();
            let result = Comment::new(comment.clone(), post_id);
            let trimmed = comment.trim();
            let grapheme_count = trimmed.graphemes(true).count();

            // Property: Comment is valid if not empty and <= 200 graphemes
            if !trimmed.is_empty() && grapheme_count <= 200 {
                prop_assert!(result.is_ok(), "Expected Ok but got {:?} for comment: {:?}", result, comment);
            } else {
                prop_assert!(result.is_err(), "Expected Err but got Ok for comment: {:?}", comment);
            }
        }

        #[test]
        fn hyphenated_uuid_format_is_validated(
            comment in r"[a-zA-Z ]{10,50}",
            // Generate strings that look like UUIDs but might be invalid
            a in "[0-9a-f]{8}",
            b in "[0-9a-f]{4}",
            c in "[0-9a-f]{4}",
            d in "[0-9a-f]{4}",
            e in "[0-9a-f]{12}",
        ) {
            let uuid_str = format!("{}-{}-{}-{}-{}", a, b, c, d, e);
            let result = Comment::new(comment, uuid_str.clone());

            // This should succeed since we're generating valid UUID format
            prop_assert!(result.is_ok(), "Valid UUID format should be accepted: {}", uuid_str);
        }
    }
}
