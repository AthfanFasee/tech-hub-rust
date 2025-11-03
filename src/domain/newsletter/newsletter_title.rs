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

impl std::fmt::Display for NewsletterTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
