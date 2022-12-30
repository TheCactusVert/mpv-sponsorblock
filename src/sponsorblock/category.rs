use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Sponsor,
    SelfPromo,
    Interaction,
    Poi,
    Intro,
    Outro,
    Preview,
    MusicOfftopic,
    Filler,
    ExclusiveAccess,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Sponsor => write!(f, "sponsor"),
            Category::SelfPromo => write!(f, "selfpromo"),
            Category::Interaction => write!(f, "interaction"),
            Category::Poi => write!(f, "poi_highlight"),
            Category::Intro => write!(f, "intro"),
            Category::Outro => write!(f, "outro"),
            Category::Preview => write!(f, "preview"),
            Category::MusicOfftopic => write!(f, "music_offtopic"),
            Category::Filler => write!(f, "filler"),
            Category::ExclusiveAccess => write!(f, "exclusive_access"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Category {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "sponsor" => Ok(Category::Sponsor),
            "selfpromo" => Ok(Category::SelfPromo),
            "interaction" => Ok(Category::Interaction),
            "poi_highlight" => Ok(Category::Poi),
            "intro" => Ok(Category::Intro),
            "outro" => Ok(Category::Outro),
            "preview" => Ok(Category::Preview),
            "music_offtopic" => Ok(Category::MusicOfftopic),
            "filler" => Ok(Category::Filler),
            "exclusive_access" => Ok(Category::ExclusiveAccess),
            _ => Err(serde::de::Error::custom("invalid category")),
        }
    }
}
