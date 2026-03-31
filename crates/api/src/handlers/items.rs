use crate::error::{json_error, validation_error};
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

/// Common SELECT columns for Item struct
const ITEM_COLS: &str = "id, list_id, title, description, completed, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline, created_at, updated_at";

/// Common SELECT columns for DateItem struct (with list info)
const DATE_ITEM_COLS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
    i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
    i.created_at, i.updated_at, l.name as list_name, l.list_type";

#[derive(Clone, Debug)]
struct ItemTemporalState {
    start_date: Option<String>,
    start_time: Option<String>,
    deadline: Option<String>,
    deadline_time: Option<String>,
    hard_deadline: Option<String>,
}

impl ItemTemporalState {
    fn from_create(body: &CreateItemRequest) -> Self {
        Self {
            start_date: body.start_date.clone(),
            start_time: body.start_time.clone(),
            deadline: body.deadline.clone(),
            deadline_time: body.deadline_time.clone(),
            hard_deadline: body.hard_deadline.clone(),
        }
    }

    fn from_item(item: &Item) -> Self {
        Self {
            start_date: item.start_date.clone(),
            start_time: item.start_time.clone(),
            deadline: item.deadline.clone(),
            deadline_time: item.deadline_time.clone(),
            hard_deadline: item.hard_deadline.clone(),
        }
    }

    fn apply_update(&mut self, body: &UpdateItemRequest) {
        apply_patch_field(&mut self.start_date, &body.start_date);
        apply_patch_field(&mut self.start_time, &body.start_time);
        apply_patch_field(&mut self.deadline, &body.deadline);
        apply_patch_field(&mut self.deadline_time, &body.deadline_time);
        apply_patch_field(&mut self.hard_deadline, &body.hard_deadline);
    }
}

#[derive(Clone, Copy, Debug)]
enum DateFieldSelector {
    All,
    One(DateField),
}

fn apply_patch_field(target: &mut Option<String>, patch: &Option<Option<String>>) {
    match patch {
        Some(Some(value)) => *target = Some(value.clone()),
        Some(None) => *target = None,
        None => {}
    }
}

fn validation_field(field: &str, code: &str) -> ValidationFieldError {
    ValidationFieldError {
        field: field.to_string(),
        code: code.to_string(),
    }
}

fn normalize_title(
    title: &str,
    field: &str,
    errors: &mut Vec<ValidationFieldError>,
) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        errors.push(validation_field(field, "required"));
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn validate_date_field(
    field: &str,
    value: Option<&str>,
    errors: &mut Vec<ValidationFieldError>,
) -> Option<chrono::NaiveDate> {
    let value = value?;
    match validate_business_date(value) {
        Ok(date) => Some(date),
        Err(DateValidationError::Invalid) => {
            errors.push(validation_field(field, "invalid_date"));
            None
        }
        Err(DateValidationError::OutOfRange) => {
            errors.push(validation_field(field, "date_out_of_range"));
            None
        }
    }
}

fn validate_time_field(
    field: &str,
    value: Option<&str>,
    errors: &mut Vec<ValidationFieldError>,
) -> bool {
    let Some(value) = value else {
        return true;
    };
    if validate_hhmm_time(value).is_ok() {
        true
    } else {
        errors.push(validation_field(field, "invalid_time"));
        false
    }
}

fn validate_item_temporal_state(state: &ItemTemporalState) -> Vec<ValidationFieldError> {
    let mut errors = Vec::new();
    let start_date = validate_date_field("start_date", state.start_date.as_deref(), &mut errors);
    let deadline = validate_date_field("deadline", state.deadline.as_deref(), &mut errors);
    let hard_deadline =
        validate_date_field("hard_deadline", state.hard_deadline.as_deref(), &mut errors);

    let start_has_valid_time =
        validate_time_field("start_time", state.start_time.as_deref(), &mut errors);
    let deadline_has_valid_time =
        validate_time_field("deadline_time", state.deadline_time.as_deref(), &mut errors);

    if state.start_time.is_some() && state.start_date.is_none() && start_has_valid_time {
        errors.push(validation_field("start_time", "time_requires_date"));
    }
    if state.deadline_time.is_some() && state.deadline.is_none() && deadline_has_valid_time {
        errors.push(validation_field("deadline_time", "time_requires_date"));
    }

    if let (Some(start_date), Some(deadline)) = (start_date, deadline)
        && start_date > deadline
    {
        errors.push(validation_field("start_date", "start_after_deadline"));
    }
    if let (Some(deadline), Some(hard_deadline)) = (deadline, hard_deadline)
        && deadline > hard_deadline
    {
        errors.push(validation_field(
            "hard_deadline",
            "hard_deadline_before_deadline",
        ));
    }

    errors
}

fn parse_date_field_selector(date_field: &str) -> std::result::Result<DateFieldSelector, Response> {
    match date_field {
        "all" => Ok(DateFieldSelector::All),
        "start_date" => Ok(DateFieldSelector::One(DateField::StartDate)),
        "deadline" => Ok(DateFieldSelector::One(DateField::Deadline)),
        "hard_deadline" => Ok(DateFieldSelector::One(DateField::HardDeadline)),
        _ => Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field("date_field", "invalid_date_field")],
        )
        .expect("build 422 response")),
    }
}

fn parse_required_query_date(
    field: &str,
    value: Option<String>,
) -> std::result::Result<chrono::NaiveDate, Response> {
    let Some(value) = value else {
        return Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field(field, "required")],
        )
        .expect("build 422 response"));
    };

    match validate_business_date(&value) {
        Ok(date) => Ok(date),
        Err(DateValidationError::Invalid) => Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field(field, "invalid_date")],
        )
        .expect("build 422 response")),
        Err(DateValidationError::OutOfRange) => Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field(field, "date_out_of_range")],
        )
        .expect("build 422 response")),
    }
}

fn relevant_date_for_item(item: &DateItem, selector: DateFieldSelector) -> Option<&str> {
    match selector {
        DateFieldSelector::All => match item.date_type.as_deref() {
            Some("start") => item.start_date.as_deref(),
            Some("hard_deadline") => item.hard_deadline.as_deref(),
            Some("deadline") => item.deadline.as_deref(),
            _ => None,
        },
        DateFieldSelector::One(DateField::StartDate) => item.start_date.as_deref(),
        DateFieldSelector::One(DateField::Deadline) => item.deadline.as_deref(),
        DateFieldSelector::One(DateField::HardDeadline) => item.hard_deadline.as_deref(),
    }
}

fn keep_item_for_day(
    item: &DateItem,
    selector: DateFieldSelector,
    target: chrono::NaiveDate,
    include_overdue: bool,
) -> bool {
    let Some(date_value) = relevant_date_for_item(item, selector) else {
        return false;
    };
    let Ok(item_date) = validate_business_date(date_value) else {
        return false;
    };

    match selector {
        DateFieldSelector::All => match item.date_type.as_deref() {
            Some("deadline") => {
                item_date == target || (include_overdue && item_date < target && !item.completed)
            }
            Some("start") | Some("hard_deadline") => item_date == target,
            _ => false,
        },
        DateFieldSelector::One(_) => {
            item_date == target || (include_overdue && item_date < target && !item.completed)
        }
    }
}

fn date_key_in_range(
    item: &DateItem,
    selector: DateFieldSelector,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Option<String> {
    let date_value = relevant_date_for_item(item, selector)?;
    let item_date = validate_business_date(date_value).ok()?;
    if item_date < from || item_date > to {
        return None;
    }
    Some(format_date(&item_date))
}

fn filter_day_summaries(
    summaries: Vec<DaySummary>,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Vec<DaySummary> {
    summaries
        .into_iter()
        .filter_map(|mut summary| {
            let parsed = validate_business_date(&summary.date).ok()?;
            if parsed < from || parsed > to {
                return None;
            }
            summary.date = format_date(&parsed);
            Some(summary)
        })
        .collect()
}

#[instrument(skip_all)]
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    let query = format!(
        "SELECT {} FROM items WHERE list_id = ?1 ORDER BY completed ASC, position ASC, created_at ASC",
        ITEM_COLS
    );
    let result = d1.prepare(&query).bind(&[list_id.into()])?.all().await?;
    let items = result.results::<Item>()?;
    Response::from_json(&items)
}

#[instrument(skip_all)]
pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_item_ownership(&d1, &id, &user_id).await? {
        return json_error("item_not_found", 404);
    }

    let query = format!(
        "SELECT {} FROM items WHERE id = ?1 AND list_id = ?2",
        ITEM_COLS
    );
    let item = d1
        .prepare(&query)
        .bind(&[id.into(), list_id.clone().into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    // Fetch list name + features for the combined response (saves a round-trip from the client)
    #[derive(serde::Deserialize)]
    struct ListRow {
        name: String,
        features: Option<String>,
    }
    let list_row = d1
        .prepare("SELECT name, (SELECT COALESCE(json_group_array(json_object('name', lf.feature_name, 'config', json(lf.config))), '[]') FROM list_features lf WHERE lf.list_id = ?1) as features FROM lists WHERE id = ?1")
        .bind(&[list_id.into()])?
        .first::<ListRow>(None)
        .await?
        .ok_or_else(|| Error::from("List not found"))?;
    let list_features: Vec<ListFeature> = list_row
        .features
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Response::from_json(&ItemDetailResponse {
        item,
        list_name: list_row.name,
        list_features,
    })
}

#[instrument(skip_all, fields(action = "create_item", item_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let body: CreateItemRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("item_id", tracing::field::display(&id));

    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    let position = next_position(&d1, "items", "list_id = ?1", &[list_id.clone().into()]).await?;

    let feature_names = get_list_features(&d1, &list_id).await?;

    let has_date_field = body.start_date.is_some()
        || body.deadline.is_some()
        || body.hard_deadline.is_some()
        || body.start_time.is_some()
        || body.deadline_time.is_some();
    let has_quantity_field = body.quantity.is_some() || body.unit.is_some();

    if let Some(err_resp) = check_item_features(&feature_names, has_date_field, has_quantity_field)?
    {
        return Ok(err_resp);
    }

    let mut validation_errors =
        validate_item_temporal_state(&ItemTemporalState::from_create(&body));
    let Some(title) = normalize_title(&body.title, "title", &mut validation_errors) else {
        return validation_error("Invalid item payload.", validation_errors);
    };
    if !validation_errors.is_empty() {
        return validation_error("Invalid item payload.", validation_errors);
    }

    let desc_val = opt_str_to_js(&body.description);
    let quantity_val: JsValue = match body.quantity {
        Some(q) => q.into(),
        None => JsValue::NULL,
    };
    let actual_quantity_val: JsValue = match body.quantity {
        Some(_) => 0i32.into(),
        None => JsValue::NULL,
    };
    let unit_val = opt_str_to_js(&body.unit);
    let start_date_val = opt_str_to_js(&body.start_date);
    let start_time_val = opt_str_to_js(&body.start_time);
    let deadline_val = opt_str_to_js(&body.deadline);
    let deadline_time_val = opt_str_to_js(&body.deadline_time);
    let hard_deadline_val = opt_str_to_js(&body.hard_deadline);

    d1.prepare(
        "INSERT INTO items (id, list_id, title, description, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )
    .bind(&[
        id.clone().into(),
        list_id.into(),
        title.into(),
        desc_val,
        position.into(),
        quantity_val,
        actual_quantity_val,
        unit_val,
        start_date_val,
        start_time_val,
        deadline_val,
        deadline_time_val,
        hard_deadline_val,
    ])?
    .run()
    .await?;

    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&select_query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create item"))?;

    let mut resp = Response::from_json(&item)?;
    resp = resp.with_status(201);
    Ok(resp)
}

/// Helper to handle Option<Option<String>> date fields in update:
/// None = don't change, Some(None) = set NULL, Some(Some(v)) = set value
fn update_nullable_str(outer: &Option<Option<String>>) -> Option<JsValue> {
    match outer {
        None => None, // don't change
        Some(None) => Some(JsValue::NULL),
        Some(Some(s)) => Some(JsValue::from(s.as_str())),
    }
}

fn check_item_features(
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

#[instrument(skip_all, fields(action = "update_item", item_id = tracing::field::Empty))]
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(&id));
    let body: UpdateItemRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let d1 = ctx.env.d1("DB")?;

    let list_id_for_features = match check_item_ownership_with_list(&d1, &id, &user_id).await? {
        Some(lid) => lid,
        None => return json_error("item_not_found", 404),
    };

    let feature_names = get_list_features(&d1, &list_id_for_features).await?;

    let has_date_field = matches!(&body.start_date, Some(Some(_)))
        || matches!(&body.deadline, Some(Some(_)))
        || matches!(&body.hard_deadline, Some(Some(_)))
        || matches!(&body.start_time, Some(Some(_)))
        || matches!(&body.deadline_time, Some(Some(_)));
    let has_quantity_field =
        body.quantity.is_some() || body.actual_quantity.is_some() || body.unit.is_some();

    if let Some(err_resp) = check_item_features(&feature_names, has_date_field, has_quantity_field)?
    {
        return Ok(err_resp);
    }

    let current_item = d1
        .prepare(format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS))
        .bind(&[id.clone().into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    let mut next_temporal_state = ItemTemporalState::from_item(&current_item);
    next_temporal_state.apply_update(&body);
    let mut validation_errors = validate_item_temporal_state(&next_temporal_state);
    let normalized_title = body
        .title
        .as_deref()
        .and_then(|title| normalize_title(title, "title", &mut validation_errors));
    if !validation_errors.is_empty() {
        return validation_error("Invalid item payload.", validation_errors);
    }

    if let Some(title) = normalized_title {
        d1.prepare("UPDATE items SET title = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[title.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(description) = &body.description {
        // Empty string is the sentinel for "clear description" (set NULL in DB).
        let desc_val: JsValue = if description.is_empty() {
            JsValue::NULL
        } else {
            description.clone().into()
        };
        d1.prepare("UPDATE items SET description = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[desc_val, id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(completed) = body.completed {
        let val: i32 = if completed { 1 } else { 0 };
        d1.prepare("UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[val.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(position) = body.position {
        d1.prepare("UPDATE items SET position = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[position.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(quantity) = body.quantity {
        d1.prepare("UPDATE items SET quantity = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[JsValue::from(quantity), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(actual) = body.actual_quantity {
        d1.prepare(
            "UPDATE items SET actual_quantity = ?1, updated_at = datetime('now') WHERE id = ?2",
        )
        .bind(&[JsValue::from(actual), id.clone().into()])?
        .run()
        .await?;

        // Auto-complete: check if actual >= target
        let row = d1
            .prepare("SELECT quantity FROM items WHERE id = ?1")
            .bind(&[id.clone().into()])?
            .first::<serde_json::Value>(None)
            .await?;
        if let Some(row) = row {
            if let Some(target) = row.get("quantity").and_then(|v| v.as_i64()) {
                let completed_val: i32 = if (actual as i64) >= target { 1 } else { 0 };
                d1.prepare(
                    "UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2",
                )
                .bind(&[JsValue::from(completed_val), id.clone().into()])?
                .run()
                .await?;
            }
        }
    }

    if let Some(unit) = &body.unit {
        d1.prepare("UPDATE items SET unit = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[JsValue::from(unit.as_str()), id.clone().into()])?
            .run()
            .await?;
    }

    // Date fields: Option<Option<String>> — None=skip, Some(None)=NULL, Some(Some(v))=set
    let date_updates: [(&str, &Option<Option<String>>); 5] = [
        ("start_date", &body.start_date),
        ("start_time", &body.start_time),
        ("deadline", &body.deadline),
        ("deadline_time", &body.deadline_time),
        ("hard_deadline", &body.hard_deadline),
    ];

    for (col, field) in &date_updates {
        if let Some(js_val) = update_nullable_str(field) {
            let sql = format!(
                "UPDATE items SET {} = ?1, updated_at = datetime('now') WHERE id = ?2",
                col
            );
            d1.prepare(&sql)
                .bind(&[js_val, id.clone().into()])?
                .run()
                .await?;
        }
    }

    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&select_query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

#[instrument(skip_all, fields(action = "delete_item", item_id = tracing::field::Empty))]
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if !check_item_ownership(&d1, &id, &user_id).await? {
        return json_error("item_not_found", 404);
    }

    d1.prepare("DELETE FROM items WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

#[instrument(skip_all, fields(action = "move_item", item_id = tracing::field::Empty))]
pub async fn move_item(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(&id));
    let body: serde_json::Value = req.json().await?;
    let target_list_id = body
        .get("target_list_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::from("Missing target_list_id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    if !check_item_ownership(&d1, &id, &user_id).await? {
        return json_error("item_not_found", 404);
    }

    if !check_ownership(&d1, "lists", &target_list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    let position = next_position(
        &d1,
        "items",
        "list_id = ?1",
        &[target_list_id.clone().into()],
    )
    .await?;

    d1.prepare(
        "UPDATE items SET list_id = ?1, position = ?2, updated_at = datetime('now') WHERE id = ?3",
    )
    .bind(&[target_list_id.into(), position.into(), id.clone().into()])?
    .run()
    .await?;

    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&select_query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

/// GET /api/items/by-date?date=YYYY-MM-DD&date_field=deadline&include_overdue=true
#[instrument(skip_all)]
pub async fn by_date(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let url = req.url()?;

    let date = match parse_required_query_date(
        "date",
        url.query_pairs()
            .find(|(k, _)| k == "date")
            .map(|(_, v)| v.to_string()),
    ) {
        Ok(date) => date,
        Err(resp) => return Ok(resp),
    };

    let include_overdue = url
        .query_pairs()
        .find(|(k, _)| k == "include_overdue")
        .map(|(_, v)| v != "false")
        .unwrap_or(true);

    let date_field_raw = url
        .query_pairs()
        .find(|(k, _)| k == "date_field")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "deadline".to_string());
    let selector = match parse_date_field_selector(&date_field_raw) {
        Ok(selector) => selector,
        Err(resp) => return Ok(resp),
    };

    let d1 = ctx.env.d1("DB")?;
    let date_str = format_date(&date);

    if matches!(selector, DateFieldSelector::All) {
        // UNION ALL across all three date fields
        let sql = format!(
            "SELECT {cols}, 'start' as date_type \
             FROM items i JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date = ?2 \
             UNION ALL \
             SELECT {cols}, 'deadline' as date_type \
             FROM items i JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 \
             AND (i.deadline = ?2{overdue}) \
             UNION ALL \
             SELECT {cols}, 'hard_deadline' as date_type \
             FROM items i JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline = ?2 \
             ORDER BY completed ASC, list_name ASC, deadline_time ASC, position ASC",
            cols = DATE_ITEM_COLS,
            overdue = if include_overdue {
                " OR (i.deadline < ?2 AND i.completed = 0)"
            } else {
                ""
            },
        );
        let result = d1
            .prepare(&sql)
            .bind(&[user_id.into(), date_str.clone().into()])?
            .all()
            .await?;
        let items = result
            .results::<DateItem>()?
            .into_iter()
            .filter(|item| keep_item_for_day(item, selector, date, include_overdue))
            .collect::<Vec<_>>();
        Response::from_json(&items)
    } else {
        // Single date field query
        let col = match selector {
            DateFieldSelector::One(field) => field.column_name(),
            DateFieldSelector::All => unreachable!("handled above"),
        };

        let sql = if include_overdue {
            format!(
                "SELECT {cols}, NULL as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND ({col} = ?2 OR ({col} < ?2 AND i.completed = 0)) \
                 ORDER BY i.completed ASC, {col} ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
            )
        } else {
            format!(
                "SELECT {cols}, NULL as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND {col} = ?2 \
                 ORDER BY i.completed ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
            )
        };

        let result = d1
            .prepare(&sql)
            .bind(&[user_id.into(), date_str.into()])?
            .all()
            .await?;
        let items = result
            .results::<DateItem>()?
            .into_iter()
            .filter(|item| keep_item_for_day(item, selector, date, include_overdue))
            .collect::<Vec<_>>();
        Response::from_json(&items)
    }
}

/// GET /api/items/calendar?from=YYYY-MM-DD&to=YYYY-MM-DD&date_field=deadline&detail=counts|full
#[instrument(skip_all)]
pub async fn calendar(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let url = req.url()?;

    let from = match parse_required_query_date(
        "from",
        url.query_pairs()
            .find(|(k, _)| k == "from")
            .map(|(_, v)| v.to_string()),
    ) {
        Ok(from) => from,
        Err(resp) => return Ok(resp),
    };

    let to = match parse_required_query_date(
        "to",
        url.query_pairs()
            .find(|(k, _)| k == "to")
            .map(|(_, v)| v.to_string()),
    ) {
        Ok(to) => to,
        Err(resp) => return Ok(resp),
    };
    if from > to {
        return validation_error(
            "Invalid query parameters.",
            vec![validation_field("from", "range_start_after_end")],
        );
    }

    let detail = url
        .query_pairs()
        .find(|(k, _)| k == "detail")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "counts".to_string());
    if detail != "counts" && detail != "full" {
        return validation_error(
            "Invalid query parameters.",
            vec![validation_field("detail", "invalid_detail")],
        );
    }

    let date_field_raw = url
        .query_pairs()
        .find(|(k, _)| k == "date_field")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "deadline".to_string());
    let selector = match parse_date_field_selector(&date_field_raw) {
        Ok(selector) => selector,
        Err(resp) => return Ok(resp),
    };

    let d1 = ctx.env.d1("DB")?;
    let from_str = format_date(&from);
    let to_str = format_date(&to);

    if detail == "full" {
        if matches!(selector, DateFieldSelector::All) {
            let sql = format!(
                "SELECT {cols}, 'start' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date >= ?2 AND i.start_date <= ?3 \
                 UNION ALL \
                 SELECT {cols}, 'deadline' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND i.deadline >= ?2 AND i.deadline <= ?3 \
                 UNION ALL \
                 SELECT {cols}, 'hard_deadline' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline >= ?2 AND i.hard_deadline <= ?3 \
                 ORDER BY completed ASC, list_name ASC, deadline_time ASC, position ASC",
                cols = DATE_ITEM_COLS,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[
                    user_id.into(),
                    from_str.clone().into(),
                    to_str.clone().into(),
                ])?
                .all()
                .await?;
            let items = result.results::<DateItem>()?;

            // Group by the date relevant to date_type
            let mut day_map: std::collections::BTreeMap<String, Vec<DateItem>> =
                std::collections::BTreeMap::new();
            for item in items {
                if let Some(date_key) = date_key_in_range(&item, selector, from, to) {
                    day_map.entry(date_key).or_default().push(item);
                }
            }
            let day_items: Vec<DayItems> = day_map
                .into_iter()
                .map(|(date, items)| DayItems { date, items })
                .collect();

            Response::from_json(&day_items)
        } else {
            let col = match selector {
                DateFieldSelector::One(field) => field.column_name(),
                DateFieldSelector::All => unreachable!("handled above"),
            };
            let sql = format!(
                "SELECT {cols}, NULL as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND {col} >= ?2 AND {col} <= ?3 \
                 ORDER BY {col} ASC, i.completed ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[
                    user_id.into(),
                    from_str.clone().into(),
                    to_str.clone().into(),
                ])?
                .all()
                .await?;
            let items = result.results::<DateItem>()?;

            let mut day_map: std::collections::BTreeMap<String, Vec<DateItem>> =
                std::collections::BTreeMap::new();
            for item in items {
                if let Some(date_key) = date_key_in_range(&item, selector, from, to) {
                    day_map.entry(date_key).or_default().push(item);
                }
            }
            let day_items: Vec<DayItems> = day_map
                .into_iter()
                .map(|(date, items)| DayItems { date, items })
                .collect();

            Response::from_json(&day_items)
        }
    } else {
        // Counts mode
        if matches!(selector, DateFieldSelector::All) {
            let sql = "SELECT date, COUNT(DISTINCT id) as total, \
                 CAST(SUM(CASE WHEN completed = 1 THEN 1 ELSE 0 END) AS INTEGER) as completed \
                 FROM ( \
                     SELECT i.id, i.start_date as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date >= ?2 AND i.start_date <= ?3 \
                     UNION ALL \
                     SELECT i.id, i.deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.deadline >= ?2 AND i.deadline <= ?3 \
                     UNION ALL \
                     SELECT i.id, i.hard_deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline >= ?2 AND i.hard_deadline <= ?3 \
                 ) GROUP BY date ORDER BY date ASC";
            let result = d1
                .prepare(sql)
                .bind(&[user_id.into(), from_str.into(), to_str.into()])?
                .all()
                .await?;
            let summaries = filter_day_summaries(result.results::<DaySummary>()?, from, to);
            Response::from_json(&summaries)
        } else {
            let col = match selector {
                DateFieldSelector::One(field) => field.column_name(),
                DateFieldSelector::All => unreachable!("handled above"),
            };
            let sql = format!(
                "SELECT {col} as date, \
                 COUNT(*) as total, \
                 CAST(SUM(i.completed) AS INTEGER) as completed \
                 FROM items i \
                 JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND {col} >= ?2 AND {col} <= ?3 \
                 GROUP BY {col} \
                 ORDER BY {col} ASC",
                col = col,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[user_id.into(), from_str.into(), to_str.into()])?
                .all()
                .await?;
            let summaries = filter_day_summaries(result.results::<DaySummary>()?, from, to);
            Response::from_json(&summaries)
        }
    }
}
