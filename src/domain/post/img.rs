#[derive(Debug)]
pub struct Img(String);

impl Img {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid image URL: cannot be empty.".to_string());
        }

        // Must be a valid HTTPS URL
        if !trimmed.starts_with("https://") {
            return Err("Invalid image URL: must be a valid HTTP or HTTPS URL.".to_string());
        }

        // Validate reasonable length for URLs
        if trimmed.len() > 2048 {
            return Err("Invalid image URL: cannot be longer than 2048 characters.".to_string());
        }

        // URLs should not contain certain characters
        let forbidden_chars = ['\0', '\n', '\r', '\t', ' '];
        if trimmed.chars().any(|c| forbidden_chars.contains(&c)) {
            return Err("Invalid image URL: contains forbidden characters.".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for Img {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Img {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
