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

impl std::fmt::Display for CommentText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
