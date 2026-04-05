use crate::error::{json_error, validation_error};
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

use super::validation::{
    ItemQuantityState, ItemTemporalState, derive_completed_from_quantity_state, normalize_title,
    validate_item_quantity_state, validate_item_temporal_state, validation_field,
};
use super::{ITEM_COLS, check_item_features, list_archived_response};

#[derive(Clone, Copy, Debug, PartialEq)]
enum ListItemsDateFieldSelector {
    All,
    One(DateField),
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ListItemsFilters {
    completed: Option<bool>,
    has_deadline: Option<bool>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
    date_field: Option<ListItemsDateFieldSelector>,
}

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.to_string())
}

fn parse_optional_bool_query(
    field: &str,
    value: Option<String>,
) -> std::result::Result<Option<bool>, Vec<ValidationFieldError>> {
    match value.as_deref() {
        None => Ok(None),
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(_) => Err(vec![validation_field(field, "invalid_boolean")]),
    }
}

fn parse_optional_query_date(
    field: &str,
    value: Option<String>,
) -> std::result::Result<Option<chrono::NaiveDate>, Vec<ValidationFieldError>> {
    let Some(value) = value else {
        return Ok(None);
    };

    match validate_business_date(&value) {
        Ok(date) => Ok(Some(date)),
        Err(DateValidationError::Invalid) => Err(vec![validation_field(field, "invalid_date")]),
        Err(DateValidationError::OutOfRange) => {
            Err(vec![validation_field(field, "date_out_of_range")])
        }
    }
}

fn parse_list_items_date_field(
    value: &str,
) -> std::result::Result<ListItemsDateFieldSelector, Vec<ValidationFieldError>> {
    match value {
        "all" => Ok(ListItemsDateFieldSelector::All),
        "start_date" => Ok(ListItemsDateFieldSelector::One(DateField::StartDate)),
        "deadline" => Ok(ListItemsDateFieldSelector::One(DateField::Deadline)),
        "hard_deadline" => Ok(ListItemsDateFieldSelector::One(DateField::HardDeadline)),
        _ => Err(vec![validation_field("date_field", "invalid_date_field")]),
    }
}

fn parse_list_items_filters(
    url: &Url,
) -> std::result::Result<ListItemsFilters, Vec<ValidationFieldError>> {
    let completed = parse_optional_bool_query("completed", query_param(url, "completed"))?;
    let has_deadline = parse_optional_bool_query("has_deadline", query_param(url, "has_deadline"))?;
    let date_from = parse_optional_query_date("date_from", query_param(url, "date_from"))?;
    let date_to = parse_optional_query_date("date_to", query_param(url, "date_to"))?;
    let date_field = match query_param(url, "date_field") {
        Some(value) => Some(parse_list_items_date_field(&value)?),
        None => None,
    };

    if date_from.is_none() && date_to.is_none() {
        if date_field.is_some() {
            return Err(vec![validation_field("date_field", "requires_date_range")]);
        }
        return Ok(ListItemsFilters {
            completed,
            has_deadline,
            date_from,
            date_to,
            date_field: None,
        });
    }

    if let (Some(date_from), Some(date_to)) = (date_from, date_to)
        && date_from > date_to
    {
        return Err(vec![validation_field("date_from", "range_start_after_end")]);
    }

    Ok(ListItemsFilters {
        completed,
        has_deadline,
        date_from,
        date_to,
        date_field: Some(date_field.unwrap_or(ListItemsDateFieldSelector::All)),
    })
}

fn build_single_field_date_clause(
    column: &str,
    from_placeholder: Option<&str>,
    to_placeholder: Option<&str>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(placeholder) = from_placeholder {
        parts.push(format!("{column} >= {placeholder}"));
    }
    if let Some(placeholder) = to_placeholder {
        parts.push(format!("{column} <= {placeholder}"));
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("({})", parts.join(" AND ")))
    }
}

fn build_date_filter_clause(
    selector: ListItemsDateFieldSelector,
    from_placeholder: Option<&str>,
    to_placeholder: Option<&str>,
) -> Option<String> {
    match selector {
        ListItemsDateFieldSelector::All => {
            let clauses = ["start_date", "deadline", "hard_deadline"]
                .into_iter()
                .filter_map(|column| {
                    build_single_field_date_clause(column, from_placeholder, to_placeholder)
                })
                .collect::<Vec<_>>();

            if clauses.is_empty() {
                None
            } else {
                Some(format!("({})", clauses.join(" OR ")))
            }
        }
        ListItemsDateFieldSelector::One(field) => {
            build_single_field_date_clause(field.column_name(), from_placeholder, to_placeholder)
        }
    }
}

fn create_item_has_date_field(body: &CreateItemRequest) -> bool {
    body.start_date.is_some()
        || body.deadline.is_some()
        || body.hard_deadline.is_some()
        || body.start_time.is_some()
        || body.deadline_time.is_some()
}

fn create_item_has_quantity_field(body: &CreateItemRequest) -> bool {
    body.quantity.is_some() || body.unit.is_some()
}

async fn fetch_item_by_id(d1: &D1Database, item_id: &str) -> Result<Item> {
    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    d1.prepare(&select_query)
        .bind(&[item_id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))
}

async fn create_items_for_list(
    d1: &D1Database,
    list_id: &str,
    feature_names: &[String],
    bodies: &[CreateItemRequest],
) -> Result<std::result::Result<Vec<Item>, Response>> {
    let base_position = next_position(d1, "items", "list_id = ?1", &[list_id.into()]).await?;
    let mut prepared = Vec::with_capacity(bodies.len());

    for (offset, body) in bodies.iter().enumerate() {
        if let Some(err_resp) = check_item_features(
            feature_names,
            create_item_has_date_field(body),
            create_item_has_quantity_field(body),
        )? {
            return Ok(Err(err_resp));
        }

        let mut validation_errors =
            validate_item_temporal_state(&ItemTemporalState::from_create(body));
        let quantity_state = ItemQuantityState::from_create(body);
        validate_item_quantity_state(&quantity_state, &mut validation_errors);
        let Some(title) = normalize_title(&body.title, "title", &mut validation_errors) else {
            return validation_error("Invalid item payload.", validation_errors).map(Err);
        };
        if !validation_errors.is_empty() {
            return validation_error("Invalid item payload.", validation_errors).map(Err);
        }

        prepared.push((
            uuid::Uuid::new_v4().to_string(),
            title,
            quantity_state,
            body.clone(),
            base_position + offset as i32,
        ));
    }

    let mut created = Vec::with_capacity(prepared.len());
    for (id, title, quantity_state, body, position) in prepared {
        let desc_val = opt_str_to_js(&body.description);
        let quantity_val: JsValue = match body.quantity {
            Some(q) => q.into(),
            None => JsValue::NULL,
        };
        let actual_quantity_val: JsValue = match quantity_state.actual_quantity {
            Some(actual_quantity) => actual_quantity.into(),
            None => JsValue::NULL,
        };
        let completed_val: i32 = if derive_completed_from_quantity_state(&quantity_state) {
            1
        } else {
            0
        };
        let unit_val = opt_str_to_js(&body.unit);
        let start_date_val = opt_str_to_js(&body.start_date);
        let start_time_val = opt_str_to_js(&body.start_time);
        let deadline_val = opt_str_to_js(&body.deadline);
        let deadline_time_val = opt_str_to_js(&body.deadline_time);
        let hard_deadline_val = opt_str_to_js(&body.hard_deadline);

        d1.prepare(
            "INSERT INTO items (id, list_id, title, description, completed, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(&[
            id.clone().into(),
            list_id.into(),
            title.into(),
            desc_val,
            completed_val.into(),
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

        created.push(fetch_item_by_id(d1, &id).await?);
    }

    Ok(Ok(created))
}

async fn set_items_completed(
    d1: &D1Database,
    user_id: &str,
    body: SetItemsCompletedRequest,
) -> Result<std::result::Result<Vec<Item>, Response>> {
    if let Err(code) = body.validate() {
        return json_error(code, 400).map(Err);
    }

    let item_ids = dedupe_ids(&body.item_ids);
    let completed_val: i32 = if body.completed { 1 } else { 0 };
    let mut items = Vec::with_capacity(item_ids.len());

    for item_id in &item_ids {
        let item_state = match get_owned_item_state(d1, item_id, user_id).await? {
            Some(item_state) => item_state,
            None => return json_error("item_not_found", 404).map(Err),
        };
        if item_state.list_archived {
            return list_archived_response().map(Err);
        }
    }

    for item_id in &item_ids {
        d1.prepare("UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[completed_val.into(), item_id.clone().into()])?
            .run()
            .await?;
        items.push(fetch_item_by_id(d1, item_id).await?);
    }

    Ok(Ok(items))
}

#[instrument(skip_all, fields(action = "list_items", list_id = tracing::field::Empty))]
pub async fn list_all(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&list_id));
    let url = req.url()?;
    let filters = match parse_list_items_filters(&url) {
        Ok(filters) => filters,
        Err(fields) => return validation_error("Invalid query parameters.", fields),
    };
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    let mut query = format!("SELECT {} FROM items WHERE list_id = ?1", ITEM_COLS);
    let mut params: Vec<JsValue> = vec![list_id.into()];

    if let Some(completed) = filters.completed {
        query.push_str(&format!(" AND completed = ?{}", params.len() + 1));
        params.push(JsValue::from(if completed { 1 } else { 0 }));
    }

    if let Some(has_deadline) = filters.has_deadline {
        query.push_str(if has_deadline {
            " AND deadline IS NOT NULL"
        } else {
            " AND deadline IS NULL"
        });
    }

    if let Some(selector) = filters.date_field {
        let from_placeholder = filters.date_from.as_ref().map(|date_from| {
            params.push(format_date(date_from).into());
            format!("?{}", params.len())
        });
        let to_placeholder = filters.date_to.as_ref().map(|date_to| {
            params.push(format_date(date_to).into());
            format!("?{}", params.len())
        });

        if let Some(date_clause) = build_date_filter_clause(
            selector,
            from_placeholder.as_deref(),
            to_placeholder.as_deref(),
        ) {
            query.push_str(" AND ");
            query.push_str(&date_clause);
        }
    }

    query.push_str(" ORDER BY position ASC, created_at ASC");
    let result = d1.prepare(&query).bind(&params)?.all().await?;
    let items = result.results::<Item>()?;
    Response::from_json(&items)
}

#[instrument(skip_all)]
pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if get_owned_item_state_in_list(&d1, &id, &list_id, &user_id)
        .await?
        .is_none()
    {
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
        .await?;
    let Some(item) = item else {
        return json_error("item_not_found", 404);
    };

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

    let Some(list_state) = get_owned_list_state(&d1, &list_id, &user_id).await? else {
        return json_error("list_not_found", 404);
    };
    if list_state.archived {
        return list_archived_response();
    }

    let feature_names = get_list_features(&d1, &list_id).await?;
    let item = match create_items_for_list(&d1, &list_id, &feature_names, &[body]).await? {
        Ok(mut items) => items
            .pop()
            .ok_or_else(|| Error::from("Failed to create item"))?,
        Err(resp) => return Ok(resp),
    };

    tracing::Span::current().record("item_id", tracing::field::display(item.id.as_str()));

    let mut resp = Response::from_json(&item)?;
    resp = resp.with_status(201);
    Ok(resp)
}

#[instrument(skip_all, fields(action = "create_items", list_id = tracing::field::Empty, item_count = tracing::field::Empty))]
pub async fn create_batch(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    tracing::Span::current().record("list_id", tracing::field::display(list_id.as_str()));
    let body: CreateItemsRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    tracing::Span::current().record("item_count", body.items.len());

    if let Err(code) = body.validate() {
        return json_error(code, 400);
    }

    let d1 = ctx.env.d1("DB")?;

    let Some(list_state) = get_owned_list_state(&d1, &list_id, &user_id).await? else {
        return json_error("list_not_found", 404);
    };
    if list_state.archived {
        return list_archived_response();
    }

    let feature_names = get_list_features(&d1, &list_id).await?;
    let items = match create_items_for_list(&d1, &list_id, &feature_names, &body.items).await? {
        Ok(items) => items,
        Err(resp) => return Ok(resp),
    };

    let mut resp = Response::from_json(&items)?;
    resp = resp.with_status(201);
    Ok(resp)
}

#[instrument(skip_all, fields(action = "update_item", item_id = tracing::field::Empty))]
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(&id));
    let body: UpdateItemRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let d1 = ctx.env.d1("DB")?;

    let item_state = match get_owned_item_state_in_list(&d1, &id, &list_id, &user_id).await? {
        Some(item_state) => item_state,
        None => return json_error("item_not_found", 404),
    };
    if item_state.list_archived {
        return list_archived_response();
    }

    let feature_names = get_list_features(&d1, &item_state.list_id).await?;

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
    let mut next_quantity_state = ItemQuantityState::from_item(&current_item);
    next_quantity_state.apply_update(&body);
    let mut validation_errors = validate_item_temporal_state(&next_temporal_state);
    validate_item_quantity_state(&next_quantity_state, &mut validation_errors);
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

    if let Some(desc_val) = nullable_string_patch_to_js(&body.description, true) {
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

    let quantity_changed = body.quantity.is_some() || body.actual_quantity.is_some();

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
    }

    if let Some(unit_val) = nullable_string_patch_to_js(&body.unit, false) {
        d1.prepare("UPDATE items SET unit = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[unit_val, id.clone().into()])?
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
        if let Some(js_val) = nullable_string_patch_to_js(field, false) {
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

    if quantity_changed {
        let completed_val: i32 = if derive_completed_from_quantity_state(&next_quantity_state) {
            1
        } else {
            0
        };
        d1.prepare("UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[completed_val.into(), id.clone().into()])?
            .run()
            .await?;
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

#[instrument(skip_all, fields(action = "set_items_completed", item_count = tracing::field::Empty))]
pub async fn set_completed(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: SetItemsCompletedRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    tracing::Span::current().record("item_count", body.item_ids.len());

    let d1 = ctx.env.d1("DB")?;
    match set_items_completed(&d1, &user_id, body).await? {
        Ok(items) => Response::from_json(&items),
        Err(resp) => Ok(resp),
    }
}

#[instrument(skip_all, fields(action = "delete_item", item_id = tracing::field::Empty))]
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    let item_state = match get_owned_item_state_in_list(&d1, &id, &list_id, &user_id).await? {
        Some(item_state) => item_state,
        None => return json_error("item_not_found", 404),
    };
    if item_state.list_archived {
        return list_archived_response();
    }

    d1.prepare("DELETE FROM items WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_filters(
        query: &str,
    ) -> std::result::Result<ListItemsFilters, Vec<ValidationFieldError>> {
        let url = Url::parse(&format!(
            "https://example.com/api/lists/list-1/items{query}"
        ))
        .expect("valid url");
        parse_list_items_filters(&url)
    }

    #[test]
    fn list_items_filters_parse_boolean_fields() {
        let filters =
            parse_filters("?completed=true&has_deadline=false").expect("filters should parse");

        assert_eq!(
            filters,
            ListItemsFilters {
                completed: Some(true),
                has_deadline: Some(false),
                date_from: None,
                date_to: None,
                date_field: None,
            }
        );
    }

    #[test]
    fn list_items_filters_reject_invalid_boolean() {
        let errors = parse_filters("?completed=yes").expect_err("should reject invalid boolean");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "completed");
        assert_eq!(errors[0].code, "invalid_boolean");
    }

    #[test]
    fn list_items_filters_default_date_field_to_all_for_open_range() {
        let filters = parse_filters("?date_from=2026-01-01").expect("filters should parse");

        assert_eq!(
            filters.date_from,
            Some(validate_business_date("2026-01-01").unwrap())
        );
        assert_eq!(filters.date_field, Some(ListItemsDateFieldSelector::All));
    }

    #[test]
    fn list_items_filters_reject_date_field_without_bounds() {
        let errors =
            parse_filters("?date_field=deadline").expect_err("should reject date field alone");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "date_field");
        assert_eq!(errors[0].code, "requires_date_range");
    }

    #[test]
    fn list_items_filters_reject_inverted_date_range() {
        let errors = parse_filters("?date_from=2026-02-01&date_to=2026-01-01")
            .expect_err("should reject invalid range");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "date_from");
        assert_eq!(errors[0].code, "range_start_after_end");
    }

    #[test]
    fn build_date_filter_clause_for_all_uses_or_across_fields() {
        let clause =
            build_date_filter_clause(ListItemsDateFieldSelector::All, Some("?2"), Some("?3"))
                .expect("clause should be built");

        assert!(clause.contains("start_date >= ?2"));
        assert!(clause.contains("deadline >= ?2"));
        assert!(clause.contains("hard_deadline <= ?3"));
        assert!(clause.contains(" OR "));
    }

    #[test]
    fn build_date_filter_clause_for_single_field_uses_selected_column() {
        let clause = build_date_filter_clause(
            ListItemsDateFieldSelector::One(DateField::Deadline),
            None,
            Some("?2"),
        )
        .expect("clause should be built");

        assert_eq!(clause, "(deadline <= ?2)");
    }
}
