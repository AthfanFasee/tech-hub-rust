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
    use proptest::prelude::*;
    use unicode_segmentation::UnicodeSegmentation;

    // Example-based tests for clear documentation
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

    // Property-based tests
    proptest! {
        #[test]
        fn names_without_forbidden_chars_and_valid_length_are_accepted(
            // Generate strings with safe characters only
            name in r"[a-zA-Z0-9 _.@#$%&*+=!?,:;'-]{1,256}"
        ) {
            prop_assert!(UserName::parse(name).is_ok());
        }

        #[test]
        fn names_with_any_forbidden_char_are_rejected(
            // Generate a name that definitely contains a forbidden character
            prefix in r"[a-zA-Z0-9]{0,10}",
            forbidden in r#"[/()<>"\\{}]"#,
            suffix in r"[a-zA-Z0-9]{0,10}"
        ) {
            let name = format!("{}{}{}", prefix, forbidden, suffix);
            prop_assert!(UserName::parse(name).is_err());
        }

        #[test]
        fn names_longer_than_256_graphemes_are_rejected(
            name in r"[a-zA-Z0-9]{257,300}"
        ) {
            prop_assert!(UserName::parse(name).is_err());
        }

        #[test]
        fn whitespace_only_names_are_rejected(
            name in r"\s{1,50}"
        ) {
            prop_assert!(UserName::parse(name).is_err());
        }

        #[test]
        fn names_with_unicode_in_valid_range_are_handled_correctly(
            // Generate strings with various Unicode characters
            name in prop::collection::vec(any::<char>(), 1..=256)
                .prop_map(|chars| chars.into_iter().collect::<String>())
        ) {
            let result = UserName::parse(name.clone());
            let trimmed = name.trim();
            let grapheme_count = trimmed.graphemes(true).count();
            let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
            let has_forbidden = trimmed.chars().any(|c| forbidden_chars.contains(&c));

            // Property: Name is valid if and only if:
            // 1. Not empty after trimming
            // 2. <= 256 graphemes
            // 3. No forbidden characters
            if !trimmed.is_empty() && grapheme_count <= 256 && !has_forbidden {
                prop_assert!(result.is_ok(), "Expected Ok but got {:?} for name: {:?}", result, name);
            } else {
                prop_assert!(result.is_err(), "Expected Err but got Ok for name: {:?}", name);
            }
        }
    }
}
