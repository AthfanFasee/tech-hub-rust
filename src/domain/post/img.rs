#[derive(Debug)]
pub struct Img(String);

impl Img {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid image URL/path: cannot be empty.".to_string());
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
