use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct Title(String);

impl Title {
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

impl AsRef<str> for Title {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
