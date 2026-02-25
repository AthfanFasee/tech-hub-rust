use std::fmt::{self, Display, Formatter};

use validator::ValidateEmail;

#[derive(Debug)]
pub struct UserEmail(String);

impl UserEmail {
    /// Returns an instance of `UserEmail` if all conditions are met.
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid email: email cannot be empty.".to_string());
        }

        // RFC 5321: 64 local + 1 @ + 255 domain = 320 characters
        if trimmed.len() > 320 {
            return Err("Invalid email: cannot be longer than 320 characters.".to_string());
        }

        if !trimmed.contains('@') {
            return Err("Invalid email: missing '@' character.".to_string());
        }

        if !trimmed.validate_email() {
            return Err(format!(
                "Invalid email: '{trimmed}' does not match the required format."
            ));
        }

        Ok(UserEmail(trimmed.to_string()))
    }
}

impl AsRef<str> for UserEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for UserEmail {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Forward to the Display implementation of the wrapped String.
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;
    use fake::{Fake, faker::internet::en::SafeEmail};
    use proptest::prelude::*;
    use rand::{SeedableRng, rngs::StdRng};

    use super::UserEmail;

    // Example-based tests for specific edge cases
    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(UserEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "athanfasee.com".to_string();
        assert_err!(UserEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(UserEmail::parse(email));
    }

    // Property-based tests
    // Define a strategy for generating valid emails
    fn valid_email_strategy() -> impl Strategy<Value = String> {
        // Generate values deterministically based on the test seed
        (0u64..1000u64).prop_map(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            SafeEmail().fake_with_rng(&mut rng)
        })
    }

    proptest! {
        #[test]
        fn valid_emails_are_parsed_successfully(email in valid_email_strategy()) {
            prop_assert!(UserEmail::parse(email).is_ok());
        }

        #[test]
        fn empty_strings_are_rejected(whitespace in r"\s*") {
            prop_assert!(UserEmail::parse(whitespace).is_err());
        }

        #[test]
        fn emails_without_at_are_rejected(
            local in "[a-z]{1,10}",
            domain in "[a-z]{1,10}"
        ) {
            let email = format!("{}{}", local, domain);
            prop_assert!(UserEmail::parse(email).is_err());
        }
    }
}
