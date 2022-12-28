use regex::Regex;

pub fn get_youtube_id<S: AsRef<str>>(path: S) -> Option<String> {
    let regex = Regex::new(
        r"https?://(?:(?:www\.|m\.|)youtube\.com|(?:www\.|)youtu\.be).*(?:/|%3D|v=|vi=)([0-9A-z-_]{11})(?:[%#?&]|$)",
    )
    .ok()?;
    let capture = regex.captures(path.as_ref())?;
    capture.get(1).map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::get_youtube_id;

    #[test]
    fn parse_youtube_id() {
        assert_eq!(
            get_youtube_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("http://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://youtube.com/v/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://www.youtube.com/v/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=20s"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://youtu.be/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://www.youtube.com/embed/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(get_youtube_id("file:///home/me/videos/some_video_file.mkv"), None);
    }
}
