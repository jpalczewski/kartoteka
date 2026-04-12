use crate::error::{json_error, validation_error};
use crate::helpers::{check_ownership, dedupe_ids};
use kartoteka_shared::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

use super::{SEARCH_ITEM_COLS, validation::validation_field};

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 100;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SearchItemsFilters {
    query: Option<String>,
    search_title: bool,
    search_description: bool,
    tag_ids: Vec<String>,
    recursive_tags: bool,
    completed: Option<bool>,
    include_archived: bool,
    limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SearchItemsCursorLast {
    rank: Option<f64>,
    completed: bool,
    updated_at: String,
    list_name: String,
    position: i32,
    id: String,
}

fn deserialize_bool_from_number<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = f64::deserialize(deserializer)?;
    Ok(value != 0.0)
}

#[derive(Debug, Deserialize)]
struct SearchItemRow {
    id: String,
    list_id: String,
    title: String,
    description: Option<String>,
    #[serde(deserialize_with = "deserialize_bool_from_number")]
    completed: bool,
    position: i32,
    quantity: Option<i32>,
    actual_quantity: Option<i32>,
    unit: Option<String>,
    start_date: Option<String>,
    start_time: Option<String>,
    deadline: Option<String>,
    deadline_time: Option<String>,
    hard_deadline: Option<String>,
    created_at: String,
    updated_at: String,
    list_name: String,
    list_type: ListType,
    #[serde(deserialize_with = "deserialize_bool_from_number")]
    list_archived: bool,
    rank: Option<f64>,
}

impl SearchItemRow {
    fn into_result(self, tag_ids: Vec<String>) -> SearchItemResult {
        SearchItemResult {
            id: self.id,
            list_id: self.list_id,
            title: self.title,
            description: self.description,
            completed: self.completed,
            position: self.position,
            quantity: self.quantity,
            actual_quantity: self.actual_quantity,
            unit: self.unit,
            start_date: self.start_date,
            start_time: self.start_time,
            deadline: self.deadline,
            deadline_time: self.deadline_time,
            hard_deadline: self.hard_deadline,
            created_at: self.created_at,
            updated_at: self.updated_at,
            list_name: self.list_name,
            list_type: self.list_type,
            list_archived: self.list_archived,
            tag_ids,
        }
    }
}

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.to_string())
}

fn repeated_query_params(url: &Url, key: &str) -> Vec<String> {
    url.query_pairs()
        .filter(|(k, _)| k == key)
        .map(|(_, value)| value.to_string())
        .collect()
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

fn parse_bool_with_default(
    field: &str,
    value: Option<String>,
    default: bool,
) -> std::result::Result<bool, Vec<ValidationFieldError>> {
    Ok(parse_optional_bool_query(field, value)?.unwrap_or(default))
}

fn parse_limit_query(value: Option<String>) -> std::result::Result<u32, Vec<ValidationFieldError>> {
    let Some(value) = value else {
        return Ok(DEFAULT_LIMIT);
    };

    let parsed = value
        .parse::<u32>()
        .map_err(|_| vec![validation_field("limit", "invalid_integer")])?;
    if parsed == 0 {
        return Err(vec![validation_field("limit", "must_be_positive")]);
    }

    Ok(parsed.min(MAX_LIMIT))
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| !ch.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn build_match_query(
    query: &str,
    search_title: bool,
    search_description: bool,
) -> std::result::Result<String, Vec<ValidationFieldError>> {
    let tokens = tokenize_query(query);
    if tokens.is_empty() {
        return Err(vec![validation_field("query", "required")]);
    }
    if !search_title && !search_description {
        return Err(vec![validation_field("query", "requires_search_field")]);
    }

    let parts = tokens
        .into_iter()
        .map(|token| {
            let escaped_token = token.replace('"', "\"\"");
            let phrase = format!("\"{escaped_token}\"");
            if search_title && search_description {
                phrase
            } else if search_title {
                format!("title : {phrase}")
            } else {
                format!("description : {phrase}")
            }
        })
        .collect::<Vec<_>>();

    Ok(parts.join(" AND "))
}

fn parse_search_items_filters(
    url: &Url,
) -> std::result::Result<SearchItemsFilters, Vec<ValidationFieldError>> {
    let query = query_param(url, "query").map(|value| value.trim().to_string());
    let query = query.filter(|value| !value.is_empty());
    let search_title =
        parse_bool_with_default("search_title", query_param(url, "search_title"), true)?;
    let search_description = parse_bool_with_default(
        "search_description",
        query_param(url, "search_description"),
        true,
    )?;
    let tag_ids = dedupe_ids(&repeated_query_params(url, "tag_id"));
    let recursive_tags =
        parse_bool_with_default("recursive_tags", query_param(url, "recursive_tags"), true)?;
    let completed = parse_optional_bool_query("completed", query_param(url, "completed"))?;
    let include_archived = parse_bool_with_default(
        "include_archived",
        query_param(url, "include_archived"),
        false,
    )?;
    let limit = parse_limit_query(query_param(url, "limit"))?;

    let mut errors = Vec::new();
    if query.is_none() && tag_ids.is_empty() && completed.is_none() {
        errors.push(validation_field("query", "required"));
    }
    if query.is_some() && !search_title && !search_description {
        errors.push(validation_field("query", "requires_search_field"));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(SearchItemsFilters {
        query,
        search_title,
        search_description,
        tag_ids,
        recursive_tags,
        completed,
        include_archived,
        limit,
    })
}

fn build_tag_cte_sql(filters: &SearchItemsFilters, params: &mut Vec<JsValue>) -> Option<String> {
    if filters.tag_ids.is_empty() {
        return None;
    }

    let start_index = params.len() + 1;
    let placeholders = filters
        .tag_ids
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", start_index + index))
        .collect::<Vec<_>>()
        .join(", ");

    for tag_id in &filters.tag_ids {
        params.push(tag_id.clone().into());
    }

    let root_sql = format!("SELECT id FROM tags WHERE user_id = ?1 AND id IN ({placeholders})");

    if filters.recursive_tags {
        Some(format!(
            "WITH RECURSIVE selected_tags(id) AS ( \
                {root_sql} \
                UNION ALL \
                SELECT t.id FROM tags t \
                JOIN selected_tags st ON t.parent_tag_id = st.id \
                WHERE t.user_id = ?1 \
            )"
        ))
    } else {
        Some(format!("WITH selected_tags(id) AS ({root_sql})"))
    }
}

fn build_search_sql(
    filters: &SearchItemsFilters,
) -> std::result::Result<(String, Vec<JsValue>, bool), Vec<ValidationFieldError>> {
    let mut params = vec![JsValue::NULL];
    let tag_cte = build_tag_cte_sql(filters, &mut params);
    let mut sql = String::new();

    if let Some(tag_cte) = tag_cte {
        sql.push_str(&tag_cte);
        sql.push(' ');
    }

    let has_text_query = if let Some(query) = filters.query.as_deref() {
        let match_query =
            build_match_query(query, filters.search_title, filters.search_description)?;
        params.push(match_query.into());
        sql.push_str(&format!(
            "SELECT {cols}, bm25(items_fts, 10.0, 5.0) as rank \
             FROM items_fts \
             JOIN items i ON i.rowid = items_fts.rowid \
             JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND items_fts MATCH ?{}",
            params.len(),
            cols = SEARCH_ITEM_COLS,
        ));
        true
    } else {
        sql.push_str(&format!(
            "SELECT {cols}, NULL as rank \
             FROM items i \
             JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1",
            cols = SEARCH_ITEM_COLS,
        ));
        false
    };

    if !filters.include_archived {
        sql.push_str(" AND l.archived = 0");
    }
    if let Some(completed) = filters.completed {
        params.push((if completed { 1 } else { 0 }).into());
        sql.push_str(&format!(" AND i.completed = ?{}", params.len()));
    }
    if !filters.tag_ids.is_empty() {
        sql.push_str(
            " AND EXISTS ( \
                SELECT 1 FROM item_tags it \
                JOIN selected_tags st ON st.id = it.tag_id \
                WHERE it.item_id = i.id \
            )",
        );
    }

    if has_text_query {
        let rank_placeholder = params.len() + 1;
        let completed_placeholder = params.len() + 2;
        let updated_at_placeholder = params.len() + 3;
        let list_name_placeholder = params.len() + 4;
        let position_placeholder = params.len() + 5;
        let id_placeholder = params.len() + 6;
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        sql.push_str(&format!(
            " AND (?{rank_placeholder} IS NULL OR ( \
                bm25(items_fts, 10.0, 5.0) > ?{rank_placeholder} \
                OR (bm25(items_fts, 10.0, 5.0) = ?{rank_placeholder} AND i.completed > ?{completed_placeholder}) \
                OR (bm25(items_fts, 10.0, 5.0) = ?{rank_placeholder} AND i.completed = ?{completed_placeholder} AND i.updated_at < ?{updated_at_placeholder}) \
                OR (bm25(items_fts, 10.0, 5.0) = ?{rank_placeholder} AND i.completed = ?{completed_placeholder} AND i.updated_at = ?{updated_at_placeholder} AND l.name > ?{list_name_placeholder}) \
                OR (bm25(items_fts, 10.0, 5.0) = ?{rank_placeholder} AND i.completed = ?{completed_placeholder} AND i.updated_at = ?{updated_at_placeholder} AND l.name = ?{list_name_placeholder} AND i.position > ?{position_placeholder}) \
                OR (bm25(items_fts, 10.0, 5.0) = ?{rank_placeholder} AND i.completed = ?{completed_placeholder} AND i.updated_at = ?{updated_at_placeholder} AND l.name = ?{list_name_placeholder} AND i.position = ?{position_placeholder} AND i.id > ?{id_placeholder}) \
            ))"
        ));
    } else {
        let completed_placeholder = params.len() + 1;
        let updated_at_placeholder = params.len() + 2;
        let list_name_placeholder = params.len() + 3;
        let position_placeholder = params.len() + 4;
        let id_placeholder = params.len() + 5;
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        params.push(JsValue::NULL);
        sql.push_str(&format!(
            " AND (?{completed_placeholder} IS NULL OR ( \
                i.completed > ?{completed_placeholder} \
                OR (i.completed = ?{completed_placeholder} AND i.updated_at < ?{updated_at_placeholder}) \
                OR (i.completed = ?{completed_placeholder} AND i.updated_at = ?{updated_at_placeholder} AND l.name > ?{list_name_placeholder}) \
                OR (i.completed = ?{completed_placeholder} AND i.updated_at = ?{updated_at_placeholder} AND l.name = ?{list_name_placeholder} AND i.position > ?{position_placeholder}) \
                OR (i.completed = ?{completed_placeholder} AND i.updated_at = ?{updated_at_placeholder} AND l.name = ?{list_name_placeholder} AND i.position = ?{position_placeholder} AND i.id > ?{id_placeholder}) \
            ))"
        ));
    }

    sql.push_str(" ORDER BY ");
    if has_text_query {
        sql.push_str("rank ASC, ");
    }
    sql.push_str("i.completed ASC, i.updated_at DESC, l.name ASC, i.position ASC, i.id ASC");

    params.push(((filters.limit + 1) as i32).into());
    sql.push_str(&format!(" LIMIT ?{}", params.len()));

    Ok((sql, params, has_text_query))
}

fn apply_cursor_values(
    params: &mut [JsValue],
    filters: &SearchItemsFilters,
    cursor: Option<&SearchItemsCursorLast>,
    has_text_query: bool,
) {
    let base = if filters.query.is_some() {
        let mut count = 2;
        if filters.completed.is_some() {
            count += 1;
        }
        if filters.tag_ids.is_empty() {
            count
        } else {
            count + filters.tag_ids.len()
        }
    } else {
        let mut count = 1;
        if filters.completed.is_some() {
            count += 1;
        }
        if filters.tag_ids.is_empty() {
            count
        } else {
            count + filters.tag_ids.len()
        }
    };

    let Some(cursor) = cursor else {
        return;
    };

    let start = base;
    if has_text_query {
        params[start] = cursor.rank.map(JsValue::from_f64).unwrap_or(JsValue::NULL);
        params[start + 1] = (if cursor.completed { 1 } else { 0 }).into();
        params[start + 2] = cursor.updated_at.clone().into();
        params[start + 3] = cursor.list_name.clone().into();
        params[start + 4] = cursor.position.into();
        params[start + 5] = cursor.id.clone().into();
    } else {
        params[start] = (if cursor.completed { 1 } else { 0 }).into();
        params[start + 1] = cursor.updated_at.clone().into();
        params[start + 2] = cursor.list_name.clone().into();
        params[start + 3] = cursor.position.into();
        params[start + 4] = cursor.id.clone().into();
    }
}

async fn fetch_tag_ids_map(
    d1: &D1Database,
    item_ids: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>> {
    use std::collections::HashMap;

    if item_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = item_ids
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let params = item_ids
        .iter()
        .cloned()
        .map(JsValue::from)
        .collect::<Vec<_>>();
    let rows = d1
        .prepare(format!(
            "SELECT item_id, tag_id FROM item_tags WHERE item_id IN ({placeholders}) ORDER BY item_id ASC, tag_id ASC"
        ))
        .bind(&params)?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let mut tag_map = HashMap::<String, Vec<String>>::new();
    for row in rows {
        let Some(item_id) = row.get("item_id").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(tag_id) = row.get("tag_id").and_then(|value| value.as_str()) else {
            continue;
        };
        tag_map
            .entry(item_id.to_string())
            .or_default()
            .push(tag_id.to_string());
    }

    Ok(tag_map)
}

fn build_search_page(
    filters: &SearchItemsFilters,
    mut rows: Vec<SearchItemRow>,
    tag_map: &mut std::collections::HashMap<String, Vec<String>>,
) -> Result<CursorPage<SearchItemResult>> {
    let next_cursor = if rows.len() > filters.limit as usize {
        rows.truncate(filters.limit as usize);
        rows.last()
            .map(|last| {
                crate::cursor::encode_cursor(
                    "search_items",
                    filters.limit,
                    filters,
                    &SearchItemsCursorLast {
                        rank: last.rank,
                        completed: last.completed,
                        updated_at: last.updated_at.clone(),
                        list_name: last.list_name.clone(),
                        position: last.position,
                        id: last.id.clone(),
                    },
                )
            })
            .transpose()
            .map_err(|e| Error::from(e.to_string()))?
    } else {
        None
    };

    let items = rows
        .into_iter()
        .map(|row| {
            let tag_ids = tag_map.remove(&row.id).unwrap_or_default();
            row.into_result(tag_ids)
        })
        .collect();

    Ok(CursorPage { items, next_cursor })
}

async fn execute_search_page(
    d1: &D1Database,
    user_id: &str,
    filters: &SearchItemsFilters,
    cursor: Option<&SearchItemsCursorLast>,
) -> Result<CursorPage<SearchItemResult>> {
    for tag_id in &filters.tag_ids {
        if !check_ownership(d1, "tags", tag_id, user_id).await? {
            return Err(Error::from("tag_not_found"));
        }
    }

    let (sql, mut params, has_text_query) =
        build_search_sql(filters).map_err(|_| Error::from("invalid_cursor"))?;
    params[0] = user_id.into();
    apply_cursor_values(&mut params, filters, cursor, has_text_query);

    let rows = d1
        .prepare(&sql)
        .bind(&params)?
        .all()
        .await?
        .results::<SearchItemRow>()?;
    let item_ids = rows
        .iter()
        .take(filters.limit as usize)
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    let mut tag_map = fetch_tag_ids_map(d1, &item_ids).await?;
    build_search_page(filters, rows, &mut tag_map)
}

pub(crate) async fn next_search_page(
    d1: &D1Database,
    user_id: &str,
    envelope: crate::cursor::PageCursorEnvelope,
) -> Result<Response> {
    let filters: SearchItemsFilters =
        serde_json::from_value(envelope.params).map_err(|_| Error::from("invalid_cursor"))?;
    let last: SearchItemsCursorLast =
        serde_json::from_value(envelope.last).map_err(|_| Error::from("invalid_cursor"))?;
    let page = match execute_search_page(d1, user_id, &filters, Some(&last)).await {
        Ok(page) => page,
        Err(err) if err.to_string() == "tag_not_found" => return json_error("tag_not_found", 404),
        Err(err) => return Err(err),
    };
    Response::from_json(&page)
}

/// GET /api/items/search
#[instrument(skip_all, fields(action = "search_items"))]
pub async fn search(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let url = req.url()?;
    let filters = match parse_search_items_filters(&url) {
        Ok(filters) => filters,
        Err(fields) => return validation_error("Invalid query parameters.", fields),
    };

    let d1 = ctx.env.d1("DB")?;
    let page = match execute_search_page(&d1, &user_id, &filters, None).await {
        Ok(page) => page,
        Err(err) if err.to_string() == "tag_not_found" => return json_error("tag_not_found", 404),
        Err(err) => return Err(err),
    };
    Response::from_json(&page)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_filters(
        query: &str,
    ) -> std::result::Result<SearchItemsFilters, Vec<ValidationFieldError>> {
        let url =
            Url::parse(&format!("https://example.com/api/items/search{query}")).expect("valid url");
        parse_search_items_filters(&url)
    }

    #[test]
    fn search_filters_require_query_or_tag() {
        let errors = parse_filters("").expect_err("filters should fail");
        assert_eq!(errors[0].field, "query");
        assert_eq!(errors[0].code, "required");
    }

    #[test]
    fn search_filters_accept_completed_only() {
        let filters = parse_filters("?completed=true").expect("filters should parse");
        assert_eq!(filters.completed, Some(true));
        assert!(filters.query.is_none());
        assert!(filters.tag_ids.is_empty());
    }

    #[test]
    fn search_filters_accept_tag_only() {
        let filters = parse_filters("?tag_id=t1&tag_id=t2").expect("filters should parse");
        assert_eq!(filters.tag_ids, vec!["t1".to_string(), "t2".to_string()]);
        assert!(filters.query.is_none());
    }

    #[test]
    fn search_filters_reject_empty_trimmed_query() {
        let errors = parse_filters("?query=%20%20%20").expect_err("filters should fail");
        assert_eq!(errors[0].field, "query");
        assert_eq!(errors[0].code, "required");
    }

    #[test]
    fn search_filters_require_enabled_search_field_for_query() {
        let errors = parse_filters("?query=milk&search_title=false&search_description=false")
            .expect_err("filters should fail");
        assert_eq!(errors[0].field, "query");
        assert_eq!(errors[0].code, "requires_search_field");
    }

    #[test]
    fn build_match_query_for_title_only_uses_column_filter() {
        let query = build_match_query("buy milk", true, false).expect("query should build");
        assert_eq!(query, "title : \"buy\" AND title : \"milk\"");
    }

    #[test]
    fn build_match_query_tokenizes_plain_text() {
        let query = build_match_query("milk, bread!", true, true).expect("query should build");
        assert_eq!(query, "\"milk\" AND \"bread\"");
    }

    #[test]
    fn build_match_query_quotes_hyphenated_terms() {
        let query = build_match_query("tag-test", true, true).expect("query should build");
        assert_eq!(query, "\"tag-test\"");
    }
}
