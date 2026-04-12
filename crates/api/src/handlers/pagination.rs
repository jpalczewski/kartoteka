use crate::cursor::decode_cursor;
use crate::error::json_error;
use crate::handlers::{containers, items, lists, search, tags};
use tracing::instrument;
use worker::*;

fn validate_cursor_limit(limit: u32) -> bool {
    limit > 0 && limit <= 100
}

#[instrument(skip_all, fields(action = "next_cursor_page"))]
pub async fn next_page(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let raw_cursor = req
        .url()?
        .query_pairs()
        .find(|(key, _)| key == "cursor")
        .map(|(_, value)| value.to_string());
    let Some(raw_cursor) = raw_cursor else {
        return json_error("invalid_cursor", 400);
    };

    let envelope = match decode_cursor(&raw_cursor) {
        Ok(envelope) => envelope,
        Err(code) => return json_error(code, 400),
    };
    if !validate_cursor_limit(envelope.limit) {
        return json_error("invalid_cursor", 400);
    }

    let d1 = ctx.env.d1("DB")?;
    match envelope.kind.as_str() {
        "lists" => {
            let last: lists::RootListsCursorLast =
                serde_json::from_value(envelope.last).map_err(|_| Error::from("invalid_cursor"))?;
            let page = lists::list_all_page(&d1, &user_id, envelope.limit, Some(&last)).await?;
            Response::from_json(&page)
        }
        "containers" => {
            let last: containers::ContainersCursorLast =
                serde_json::from_value(envelope.last).map_err(|_| Error::from("invalid_cursor"))?;
            let page =
                containers::list_all_page(&d1, &user_id, envelope.limit, Some(&last)).await?;
            Response::from_json(&page)
        }
        "tags" => {
            let last: tags::TagsCursorLast =
                serde_json::from_value(envelope.last).map_err(|_| Error::from("invalid_cursor"))?;
            let page = tags::list_all_page(&d1, &user_id, envelope.limit, Some(&last)).await?;
            Response::from_json(&page)
        }
        "list_items" => items::next_list_items_page(&d1, &user_id, envelope).await,
        "search_items" => items::next_search_page(&d1, &user_id, envelope).await,
        "search_entities" => search::next_search_page(&d1, &user_id, envelope).await,
        _ => json_error("invalid_cursor", 400),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_cursor_limit;

    #[test]
    fn validate_cursor_limit_rejects_zero() {
        assert!(!validate_cursor_limit(0));
    }

    #[test]
    fn validate_cursor_limit_accepts_max_limit() {
        assert!(validate_cursor_limit(100));
    }

    #[test]
    fn validate_cursor_limit_rejects_above_max() {
        assert!(!validate_cursor_limit(101));
    }
}
