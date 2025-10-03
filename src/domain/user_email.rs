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

impl std::fmt::Display for UserEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Forward to the Display implementation of the wrapped String.
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::UserEmail;
    use claims::assert_err;
    use fake::Fake;
    use fake::faker::internet::en::SafeEmail;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

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

    // A wrapper around a valid email string for testing
    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    // Implement Arbitrary so QuickCheck can generate random ValidEmailFixture values
    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            // Create a deterministic random number generator
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            // Generate a realistic fake email address
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    // Property-based test: randomly generated valid emails should parse successfully
    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        UserEmail::parse(valid_email.0).is_ok()
    }
}
