mod post_img;
mod post_text;
mod post_title;
mod requests;
mod types;

pub use post_img::PostImg;
pub use post_text::PostText;
pub use post_title::PostTitle;
pub use requests::*;
pub use types::*;

#[derive(Debug)]
pub struct Post {
    pub title: PostTitle,
    pub text: PostText,
    pub img: PostImg,
}

impl Post {
    pub fn new(title: String, text: String, img: String) -> Result<Self, String> {
        Ok(Self {
            title: PostTitle::parse(title)?,
            text: PostText::parse(text)?,
            img: PostImg::parse(img)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Post;
    use claims::assert_ok;
    use proptest::prelude::*;

    #[test]
    fn valid_post_is_accepted() {
        let result = Post::new(
            "A Valid Title".into(),
            "This is the posts body.".into(),
            "https://cdn.example.com/images/abc123.jpg".into(),
        );
        assert_ok!(result);
    }

    proptest! {
        #[test]
        fn all_three_fields_must_be_valid_together(
            title in r"[a-zA-Z][a-zA-Z0-9 ]{0,99}",
            text in r"[a-zA-Z0-9][a-zA-Z0-9 .!?]{0,499}",
            domain in r"[a-z0-9.-]{3,30}",
            path in r"[a-zA-Z0-9/_.-]{1,30}",
        ) {
            let img = format!("https://{}/{}", domain, path);
            let result = Post::new(title, text, img);
            prop_assert!(result.is_ok());
        }
    }
}
