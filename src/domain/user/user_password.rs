use secrecy::Secret;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct UserPassword(Secret<String>);

impl UserPassword {
    // Returns an instance of `UserPassword` if all conditions are met.
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid user password: cannot be empty or whitespace.".to_string());
        }

        let length = trimmed.graphemes(true).count();

        if length < 8 {
            return Err("Invalid user password: must be at least 8 characters long.".to_string());
        }

        if length > 128 {
            return Err("Invalid user password: cannot be longer than 128 characters.".to_string());
        }

        // Once validated, store it secretly
        Ok(Self(Secret::new(trimmed.to_string())))
    }

    pub fn into_secret(self) -> Secret<String> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::UserPassword;
    use claims::{assert_err, assert_ok};

    #[test]
    fn passwords_shorter_than_8_graphemes_are_rejected() {
        let password = "1234567".to_string();
        assert_err!(UserPassword::parse(password));
    }

    #[test]
    fn passwords_longer_than_128_graphemes_are_rejected() {
        let password = "a".repeat(129);
        assert_err!(UserPassword::parse(password));
    }

    #[test]
    fn an_empty_string_is_rejected() {
        let password = "".to_string();
        assert_err!(UserPassword::parse(password));
    }

    #[test]
    fn whitespace_only_passwords_are_rejected() {
        let password = " ".repeat(10);
        assert_err!(UserPassword::parse(password));
    }

    #[test]
    fn a_password_with_8_graphemes_is_valid() {
        let password = "12345678".to_string();
        assert_ok!(UserPassword::parse(password));
    }

    #[test]
    fn a_password_with_symbols_is_valid() {
        let password = r#"p@$$w0rd<>(){}"#.to_string();
        assert_ok!(UserPassword::parse(password));
    }

    #[test]
    fn a_128_grapheme_password_is_valid() {
        let password = "x".repeat(128);
        assert_ok!(UserPassword::parse(password));
    }
}
