use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Skip,
    Mute,
    Full,
    Poi,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Skip => write!(f, "skip"),
            Action::Mute => write!(f, "mute"),
            Action::Full => write!(f, "full"),
            Action::Poi => write!(f, "poi"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "skip" => Ok(Action::Skip),
            "mute" => Ok(Action::Mute),
            "full" => Ok(Action::Full),
            "poi" => Ok(Action::Poi),
            _ => Err(serde::de::Error::custom("invalid action")),
        }
    }
}
