mod comment_text;
mod types;

pub use comment_text::CommentText;
pub use types::*;
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
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;
    use uuid::Uuid;

    use super::Comment;

    // Example-based tests
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

    // Property-based tests
    proptest! {
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
        fn hyphenated_uuid_format_is_validated(
            comment in r"[a-zA-Z ]{10,50}",
            a in "[0-9a-f]{8}",
            b in "[0-9a-f]{4}",
            c in "[0-9a-f]{4}",
            d in "[0-9a-f]{4}",
            e in "[0-9a-f]{12}",
        ) {
            let uuid_str = format!("{}-{}-{}-{}-{}", a, b, c, d, e);
            let result = Comment::new(comment, uuid_str.clone());

            // This generates VALID UUIDs (proper format with hex chars)
            // So it should always succeed
            prop_assert!(result.is_ok(), "Valid UUID format should be accepted: {}", uuid_str);
        }
    }
}
