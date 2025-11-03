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

impl std::fmt::Display for NewsletterText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
