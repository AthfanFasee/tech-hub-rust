mod get_all_posts;
mod img;
mod text;
mod title;
mod types;

pub use get_all_posts::*;
pub use img::Img;
pub use text::Text;
pub use title::Title;
pub use types::*;

#[derive(Debug)]
pub struct Post {
    pub title: Title,
    pub text: Text,
    pub img: Img,
}

impl Post {
    pub fn new(title: String, text: String, img: String) -> Result<Self, String> {
        Ok(Self {
            title: Title::parse(title)?,
            text: Text::parse(text)?,
            img: Img::parse(img)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Post;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    // Example-based tests
    #[test]
    fn empty_title_is_rejected() {
        let result = Post::new(
            "".into(),
            "some text".into(),
            "https://example.com/image.png".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn long_title_is_rejected() {
        let long_title = "a".repeat(101);
        let result = Post::new(
            long_title,
            "text".into(),
            "https://example.com/img.png".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_is_rejected() {
        let result = Post::new(
            "12345".into(),
            "some text".into(),
            "https://example.com/image.png".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_and_spaces_is_rejected() {
        let result = Post::new(
            "123 456".into(),
            "some text".into(),
            "https://example.com/image.png".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn title_with_numbers_and_letters_is_accepted() {
        let result = Post::new(
            "Post123".into(),
            "some text".into(),
            "https://example.com/image.png".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn title_with_letters_and_numbers_is_accepted() {
        let result = Post::new(
            "123Post".into(),
            "some text".into(),
            "https://example.com/image.png".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn empty_text_is_rejected() {
        let result = Post::new(
            "Valid Title".into(),
            "".into(),
            "https://example.com/image.png".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn empty_img_is_rejected() {
        let result = Post::new("Valid Title".into(), "some text".into(), "".into());
        assert_err!(result);
    }

    #[test]
    fn img_without_http_protocol_is_rejected() {
        let result = Post::new(
            "Valid Title".into(),
            "some text".into(),
            "storage/images/abc123".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn img_with_forbidden_chars_is_rejected() {
        let result = Post::new(
            "Valid Title".into(),
            "some text".into(),
            "https://example.com/path\nwith\nnewlines".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn img_with_spaces_is_rejected() {
        let result = Post::new(
            "Valid Title".into(),
            "some text".into(),
            "https://example.com/path with spaces".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn valid_post_with_https_url_is_accepted() {
        let result = Post::new(
            "A Valid Title".into(),
            "This is the posts body.".into(),
            "https://cdn.example.com/images/abc123.jpg".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn valid_post_with_http_url_is_accepted() {
        let result = Post::new(
            "A Valid Title".into(),
            "This is the posts body.".into(),
            "https://storage.example.com/bucket/xyz789".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn valid_post_with_cdn_url_is_accepted() {
        let result = Post::new(
            "A Valid Title".into(),
            "This is the posts body.".into(),
            "https://d1a2b3c4.cloudfront.net/images/post_123".into(),
        );
        assert_ok!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn valid_titles_with_valid_length_are_accepted(
            title in r"[a-zA-Z][a-zA-Z0-9 ]{0,99}",
        ) {
            let result = Post::new(title, "Valid text".into(), "https://example.com/image.png".into());
            prop_assert!(result.is_ok());
        }

        #[test]
        fn titles_longer_than_100_chars_are_rejected(
            title in r"[a-zA-Z0-9]{101,150}",
        ) {
            let result = Post::new(title, "Valid text".into(), "https://example.com/image.png".into());
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_titles_are_rejected(
            title in r"\s{1,50}",
        ) {
            let result = Post::new(title, "Valid text".into(), "https://example.com/image.png".into());
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_are_rejected(
            title in r"[0-9]{1,50}",
        ) {
            let result = Post::new(title, "Valid text".into(), "https://example.com/image.png".into());
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_with_spaces_are_rejected(
            num1 in r"[0-9]{1,20}",
            num2 in r"[0-9]{1,20}",
        ) {
            let title = format!("{} {}", num1, num2);
            let result = Post::new(title, "Valid text".into(), "https://example.com/image.png".into());
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_text_is_rejected(
            text in r"\s{1,50}",
        ) {
            let result = Post::new("Valid Title".into(), text, "https://example.com/image.png".into());
            prop_assert!(result.is_err());
        }

        #[test]
        fn valid_https_urls_are_accepted(
            domain in r"[a-z0-9.-]{3,50}",
            path in r"[a-zA-Z0-9/_.-]{1,100}",
        ) {
            let img = format!("https://{}/{}", domain, path);
            let result = Post::new("Valid Title".into(), "Valid text".into(), img);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn valid_http_urls_are_accepted(
            domain in r"[a-z0-9.-]{3,50}",
            path in r"[a-zA-Z0-9/_.-]{1,100}",
        ) {
            let img = format!("https://{}/{}", domain, path);
            let result = Post::new("Valid Title".into(), "Valid text".into(), img);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn non_url_paths_are_rejected(
            path in r"[a-zA-Z0-9/_-]{1,50}",
        ) {
            // Paths without https:// or https:// should be rejected
            let result = Post::new("Valid Title".into(), "Valid text".into(), path);
            prop_assert!(result.is_err());
        }

        #[test]
        fn urls_with_spaces_are_rejected(
            domain in r"[a-z]{3,20}",
        ) {
            let img = format!("https://{}.com/path with spaces", domain);
            let result = Post::new("Valid Title".into(), "Valid text".into(), img);
            prop_assert!(result.is_err());
        }

        #[test]
        fn all_three_fields_must_be_valid_together(
            // Title must start with a letter to ensure it's not only numbers
            title in r"[a-zA-Z][a-zA-Z0-9 ]{0,99}",
            text in r"[a-zA-Z0-9][a-zA-Z0-9 .!?]{0,499}",
            domain in r"[a-z0-9.-]{3,30}",
            path in r"[a-zA-Z0-9/_.-]{1,30}",
        ) {
            let img = format!("https://{}/{}", domain, path);
            let result = Post::new(title, text, img);
            // If all fields are valid individually, the post should be valid
            prop_assert!(result.is_ok());
        }
    }
}
