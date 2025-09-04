use validator::ValidateEmail;

#[derive(Debug)]
pub struct UserEmail(String);
impl UserEmail {
    pub fn parse(s: String) -> Result<UserEmail, String> {
       // ValidateEmail trait is implemented for String
        if s.validate_email() {
           Ok(UserEmail(s))
       } else {
           Err(format!("Email is invalid: {s}"))
       }
    }
}
impl AsRef<str> for UserEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::UserEmail;
    use claims::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

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
