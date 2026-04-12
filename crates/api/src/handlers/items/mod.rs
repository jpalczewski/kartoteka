mod calendar;
mod crud;
mod placement;
mod search;
mod validation;

pub use calendar::{by_date, calendar};
pub(crate) use crud::next_list_items_page;
pub use crud::{create, create_batch, delete, get_one, list_all, set_completed, update};
pub use placement::{move_batch, move_item, reorder, set_placement};
pub(crate) use search::next_search_page;
pub use search::search;

use crate::error::json_error;
use kartoteka_shared::*;
use worker::*;

pub(super) const ITEM_COLS: &str = "id, list_id, title, description, completed, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline, created_at, updated_at";

pub(super) const DATE_ITEM_COLS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
    i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
    i.created_at, i.updated_at, l.name as list_name, l.list_type";

pub(super) const SEARCH_ITEM_COLS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
    i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
    i.created_at, i.updated_at, l.name as list_name, l.list_type, l.archived as list_archived";

pub(super) const MAX_ITEM_TITLE_LENGTH: usize = 255;

pub(super) fn list_archived_response() -> worker::Result<Response> {
    json_error("list_archived", 409)
}

pub(super) fn check_item_features(
    feature_names: &[String],
    has_date_field: bool,
    has_quantity_field: bool,
) -> worker::Result<Option<Response>> {
    if has_date_field && !feature_names.iter().any(|f| f == FEATURE_DEADLINES) {
        return Ok(Some(
            Response::from_json(&serde_json::json!({
                "error": "feature_required",
                "feature": "deadlines",
                "message": "This list does not have the 'deadlines' feature enabled. Enable it in list settings or retry without date fields."
            }))
            .map(|r| r.with_status(422))?,
        ));
    }
    if has_quantity_field && !feature_names.iter().any(|f| f == FEATURE_QUANTITY) {
        return Ok(Some(
            Response::from_json(&serde_json::json!({
                "error": "feature_required",
                "feature": "quantity",
                "message": "This list does not have the 'quantity' feature enabled. Enable it in list settings or retry without quantity fields."
            }))
            .map(|r| r.with_status(422))?,
        ));
    }
    Ok(None)
}
