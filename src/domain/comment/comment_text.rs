#[derive(Debug)]
pub struct CommentText(String);

impl CommentText {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err("Invalid comment: cannot be empty.".to_string());
        }
        if trimmed.len() > 200 {
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

impl std::fmt::Display for CommentText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::CommentText;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_comment_less_than_200_long_is_valid() {
        let comment = "a".repeat(199);
        assert_ok!(CommentText::parse(comment));
    }

    #[test]
    fn a_comment_longer_than_200_long_is_rejected() {
        let comment = "a".repeat(201);
        assert_err!(CommentText::parse(comment));
    }

    #[test]
    fn whitespace_only_comments_are_rejected() {
        let comment = " ".to_string();
        assert_err!(CommentText::parse(comment));
    }

    #[test]
    fn empty_string_is_rejected() {
        let comment = "".to_string();
        assert_err!(CommentText::parse(comment));
    }
}
