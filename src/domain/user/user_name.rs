use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct UserName(String);

impl UserName {
    /// Returns an instance of `UserName` if all conditions are met.
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid user name: cannot be empty or whitespace.".to_string());
        }

        if trimmed.graphemes(true).count() > 256 {
            return Err("Invalid user name: cannot be longer than 256 characters.".to_string());
        }

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        if trimmed.chars().any(|c| forbidden_characters.contains(&c)) {
            return Err("Invalid user name: contains forbidden characters. The following are not allowed: / ( ) \" < > \\ { }".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UserName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::UserName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ё".repeat(256);
        assert_ok!(UserName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(UserName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Athfan Fasee".to_string();
        assert_ok!(UserName::parse(name));
    }
}
