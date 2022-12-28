mod action;
mod category;
mod segment;

pub use action::Action;
pub use category::Category;
pub use segment::{Segment, Segments};

use reqwest::{StatusCode, Url};

pub async fn fetch<C, A>(
    server_address: Url,
    privacy_api: bool,
    id: String,
    categories: C,
    action_types: A,
) -> Option<Segments>
where
    C: IntoIterator<Item = Category>,
    A: IntoIterator<Item = Action>,
{
    let segments = if privacy_api {
        Segment::fetch_with_privacy(server_address, id, categories, action_types).await
    } else {
        Segment::fetch(server_address, id, categories, action_types).await
    };

    match segments {
        Ok(v) => Some(v),
        Err(e) if e.status() == Some(StatusCode::NOT_FOUND) => None,
        Err(e) => {
            log::error!("Failed to get segments: {}", e);
            None
        }
    }
}
