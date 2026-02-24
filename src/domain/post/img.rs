use std::fmt;
use std::fmt::{Display, Formatter};

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

impl Display for Img {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::Img;
    use claims::assert_err;
    use proptest::prelude::*;

    // Example-based tests
    #[test]
    fn empty_img_is_rejected() {
        let result = Img::parse("".into());
        assert_err!(result);
    }

    #[test]
    fn img_without_http_protocol_is_rejected() {
        let result = Img::parse("storage/images/abc123".into());
        assert_err!(result);
    }

    #[test]
    fn img_with_forbidden_chars_is_rejected() {
        let result = Img::parse("https://example.com/path\nwith\nnewlines".into());
        assert_err!(result);
    }

    #[test]
    fn img_with_spaces_is_rejected() {
        let result = Img::parse("https://example.com/path with spaces".into());
        assert_err!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn valid_https_urls_are_accepted(
            domain in r"[a-z0-9.-]{3,50}",
            path in r"[a-zA-Z0-9/_.-]{1,100}",
        ) {
            let img = format!("https://{}/{}", domain, path);
            let result = Img::parse(img);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn valid_http_urls_are_accepted(
            domain in r"[a-z0-9.-]{3,50}",
            path in r"[a-zA-Z0-9/_.-]{1,100}",
        ) {
            let img = format!("https://{}/{}", domain, path);
            let result = Img::parse(img);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn non_url_paths_are_rejected(
            path in r"[a-zA-Z0-9/_-]{1,50}",
        ) {
            // Paths without https:// or https:// should be rejected
            let result = Img::parse(path);
            prop_assert!(result.is_err());
        }

        #[test]
        fn urls_with_spaces_are_rejected(
            domain in r"[a-z]{3,20}",
        ) {
            let img = format!("https://{}.com/path with spaces", domain);
            let result = Img::parse(img);
            prop_assert!(result.is_err());
        }
    }
}
