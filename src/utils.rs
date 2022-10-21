use std::time::Duration;

use curl::easy::Easy;
use regex::Regex;

pub fn get_data(url: &str, timeout: Duration) -> Result<Vec<u8>, curl::Error> {
    let mut buf = Vec::new();
    let mut handle = Easy::new();

    handle.timeout(timeout)?;

    handle.url(url)?;
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        })?;
        transfer.perform()?;
    }

    Ok(buf)
}

pub fn get_youtube_id(path: &str) -> Option<String> {
    // I don't uderstand this crap but it's working (almost)
    let regex = Regex::new(r"https?://(?:www\.)?(?:youtu\.be|youtube\.com|piped\.kavin\.rocks)/(?:v/|watch\?v=|embed/)?([A-Za-z0-9-_]{11})").ok()?;
    let capture = regex.captures(path)?;
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
        assert_eq!(
            get_youtube_id("https://piped.kavin.rocks/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(get_youtube_id("file:///home/me/videos/some_video_file.mkv"), None);
    }
}
