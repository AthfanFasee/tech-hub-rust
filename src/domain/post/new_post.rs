use crate::domain::post::img::Img;
use crate::domain::post::text::Text;
use crate::domain::post::title::Title;

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
    use crate::domain::post::new_post::Post;

    #[test]
    fn empty_title_is_rejected() {
        let result = Post::new("".into(), "some text".into(), "image.png".into());
        assert!(result.is_err());
    }

    #[test]
    fn long_title_is_rejected() {
        let long_title = "a".repeat(101);
        let result = Post::new(long_title, "text".into(), "img.png".into());
        assert!(result.is_err());
    }

    #[test]
    fn valid_post_is_accepted() {
        let result = Post::new(
            "A Valid Title".into(),
            "This is the post body.".into(),
            "img.png".into(),
        );
        assert!(result.is_ok());
    }
}
