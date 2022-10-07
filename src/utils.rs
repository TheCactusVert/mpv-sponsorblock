use curl::easy::Easy;
use regex::Regex;

pub fn get_data(url: &str) -> Result<Vec<u8>, curl::Error> {
    let mut buf = Vec::new();
    let mut handle = Easy::new();

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
    let regexes = [
        Regex::new(r"https?://youtu%.be/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"https?://w?w?w?%.?youtube%.com/v/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/watch.*[?&]v=([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/embed/([A-Za-z0-9-_]+).*").unwrap(),
    ];

    regexes
        .into_iter()
        .filter_map(|r| r.captures(path))
        .find_map(|c| c.get(1).map(|m| m.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    use super::get_youtube_id;

    #[test]
    fn test_yt_id() {
        assert_eq!(
            get_youtube_id("https://youtu.be/dQw4w9WgXcQ".to_string()),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://youtube.com/v/dQw4w9WgXcQ".to_string()),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://www.youtube.com/v/dQw4w9WgXcQ".to_string()),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://youtu.be/watch?v=dQw4w9WgXcQ".to_string()),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://www.youtube.com/embed/dQw4w9WgXcQ".to_string()),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            get_youtube_id("https://piped.kavin.rocks/watch?v=dQw4w9WgXcQ".to_string()),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(get_youtube_id("my_video.mkv".to_string()), None);
    }
}
