use crate::cursor::{PageCursorEnvelope, encode_cursor};
use crate::error::validation_error;
use kartoteka_shared::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 100;

fn validation_field(field: &str, code: &str) -> ValidationFieldError {
    ValidationFieldError {
        field: field.to_string(),
        code: code.to_string(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SearchEntitiesFilters {
    query: String,
    limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SearchEntitiesCursorLast {
    rank: f64,
    updated_at: String,
    name: String,
    entity_type: SearchEntityType,
    id: String,
}

#[derive(Debug, Deserialize)]
struct SearchEntityRow {
    entity_type: SearchEntityType,
    id: String,
    name: String,
    description: Option<String>,
    updated_at: String,
    list_id: Option<String>,
    list_name: Option<String>,
    list_type: Option<ListType>,
    archived: Option<serde_json::Value>,
    completed: Option<serde_json::Value>,
    container_id: Option<String>,
    parent_list_id: Option<String>,
    parent_container_id: Option<String>,
    status: Option<ContainerStatus>,
    rank: f64,
}

impl SearchEntityRow {
    fn into_result(self) -> Result<SearchEntityResult> {
        let archived = match self.archived {
            Some(serde_json::Value::Bool(value)) => Some(value),
            Some(serde_json::Value::Number(value)) => Some(value.as_f64().unwrap_or(0.0) != 0.0),
            _ => None,
        };
        let completed = match self.completed {
            Some(serde_json::Value::Bool(value)) => Some(value),
            Some(serde_json::Value::Number(value)) => Some(value.as_f64().unwrap_or(0.0) != 0.0),
            _ => None,
        };

        Ok(SearchEntityResult {
            entity_type: self.entity_type,
            id: self.id,
            name: self.name,
            description: self.description,
            updated_at: self.updated_at,
            list_id: self.list_id,
            list_name: self.list_name,
            list_type: self.list_type,
            archived,
            completed,
            container_id: self.container_id,
            parent_list_id: self.parent_list_id,
            parent_container_id: self.parent_container_id,
            status: self.status,
        })
    }
}

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.to_string())
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

fn build_match_query(query: &str) -> std::result::Result<String, Vec<ValidationFieldError>> {
    let tokens = tokenize_query(query);
    if tokens.is_empty() {
        return Err(vec![validation_field("query", "required")]);
    }

    Ok(tokens
        .into_iter()
        .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND "))
}

fn parse_search_entities_filters(
    url: &Url,
) -> std::result::Result<SearchEntitiesFilters, Vec<ValidationFieldError>> {
    let query = query_param(url, "query")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| vec![validation_field("query", "required")])?;
    let limit = parse_limit_query(query_param(url, "limit"))?;

    Ok(SearchEntitiesFilters { query, limit })
}

fn base_search_union_sql() -> &'static str {
    "SELECT \
        'item' AS entity_type, \
        i.id AS id, \
        i.title AS name, \
        i.description AS description, \
        i.updated_at AS updated_at, \
        i.list_id AS list_id, \
        l.name AS list_name, \
        l.list_type AS list_type, \
        l.archived AS archived, \
        i.completed AS completed, \
        l.container_id AS container_id, \
        NULL AS parent_list_id, \
        NULL AS parent_container_id, \
        NULL AS status, \
        bm25(items_fts, 10.0, 5.0) AS rank \
     FROM items_fts \
     JOIN items i ON i.rowid = items_fts.rowid \
     JOIN lists l ON l.id = i.list_id \
     WHERE l.user_id = ?1 AND l.archived = 0 AND items_fts MATCH ?2 \
     UNION ALL \
     SELECT \
        'list' AS entity_type, \
        l.id AS id, \
        l.name AS name, \
        l.description AS description, \
        l.updated_at AS updated_at, \
        NULL AS list_id, \
        NULL AS list_name, \
        l.list_type AS list_type, \
        l.archived AS archived, \
        NULL AS completed, \
        l.container_id AS container_id, \
        l.parent_list_id AS parent_list_id, \
        NULL AS parent_container_id, \
        NULL AS status, \
        bm25(lists_fts, 10.0, 5.0) AS rank \
     FROM lists_fts \
     JOIN lists l ON l.rowid = lists_fts.rowid \
     WHERE l.user_id = ?1 AND l.archived = 0 AND lists_fts MATCH ?2 \
     UNION ALL \
     SELECT \
        'container' AS entity_type, \
        c.id AS id, \
        c.name AS name, \
        c.description AS description, \
        c.updated_at AS updated_at, \
        NULL AS list_id, \
        NULL AS list_name, \
        NULL AS list_type, \
        NULL AS archived, \
        NULL AS completed, \
        NULL AS container_id, \
        NULL AS parent_list_id, \
        c.parent_container_id AS parent_container_id, \
        c.status AS status, \
        bm25(containers_fts, 10.0, 5.0) AS rank \
     FROM containers_fts \
     JOIN containers c ON c.rowid = containers_fts.rowid \
     WHERE c.user_id = ?1 AND containers_fts MATCH ?2"
}

fn build_search_sql(_limit: u32) -> String {
    format!(
        "SELECT * FROM ({union_sql}) search_results \
         WHERE (?3 IS NULL OR ( \
            search_results.rank > ?3 \
            OR (search_results.rank = ?3 AND search_results.updated_at < ?4) \
            OR (search_results.rank = ?3 AND search_results.updated_at = ?4 AND LOWER(search_results.name) > ?5) \
            OR (search_results.rank = ?3 AND search_results.updated_at = ?4 AND LOWER(search_results.name) = ?5 AND search_results.entity_type > ?6) \
            OR (search_results.rank = ?3 AND search_results.updated_at = ?4 AND LOWER(search_results.name) = ?5 AND search_results.entity_type = ?6 AND search_results.id > ?7) \
         )) \
         ORDER BY search_results.rank ASC, search_results.updated_at DESC, LOWER(search_results.name) ASC, search_results.entity_type ASC, search_results.id ASC \
         LIMIT ?8",
        union_sql = base_search_union_sql(),
    )
}

fn build_search_page(
    filters: &SearchEntitiesFilters,
    mut rows: Vec<SearchEntityRow>,
) -> Result<CursorPage<SearchEntityResult>> {
    let next_cursor = if rows.len() > filters.limit as usize {
        rows.truncate(filters.limit as usize);
        rows.last()
            .map(|last| {
                encode_cursor(
                    "search_entities",
                    filters.limit,
                    filters,
                    &SearchEntitiesCursorLast {
                        rank: last.rank,
                        updated_at: last.updated_at.clone(),
                        name: last.name.to_lowercase(),
                        entity_type: last.entity_type.clone(),
                        id: last.id.clone(),
                    },
                )
            })
            .transpose()
            .map_err(|err| Error::from(err.to_string()))?
    } else {
        None
    };

    let items = rows
        .into_iter()
        .map(SearchEntityRow::into_result)
        .collect::<Result<Vec<_>>>()?;

    Ok(CursorPage { items, next_cursor })
}

async fn execute_search_page(
    d1: &D1Database,
    user_id: &str,
    filters: &SearchEntitiesFilters,
    cursor: Option<&SearchEntitiesCursorLast>,
) -> Result<CursorPage<SearchEntityResult>> {
    let match_query =
        build_match_query(&filters.query).map_err(|_| Error::from("invalid_cursor"))?;
    let sql = build_search_sql(filters.limit);
    let mut params = vec![
        user_id.into(),
        match_query.into(),
        JsValue::NULL,
        JsValue::NULL,
        JsValue::NULL,
        JsValue::NULL,
        JsValue::NULL,
        ((filters.limit + 1) as i32).into(),
    ];

    if let Some(cursor) = cursor {
        params[2] = cursor.rank.into();
        params[3] = cursor.updated_at.clone().into();
        params[4] = cursor.name.clone().into();
        params[5] = serde_json::to_string(&cursor.entity_type)
            .map_err(|err| Error::from(err.to_string()))?
            .trim_matches('"')
            .to_string()
            .into();
        params[6] = cursor.id.clone().into();
    }

    let rows = d1
        .prepare(&sql)
        .bind(&params)?
        .all()
        .await?
        .results::<SearchEntityRow>()?;

    build_search_page(filters, rows)
}

pub(crate) async fn next_search_page(
    d1: &D1Database,
    user_id: &str,
    envelope: PageCursorEnvelope,
) -> Result<Response> {
    let filters: SearchEntitiesFilters =
        serde_json::from_value(envelope.params).map_err(|_| Error::from("invalid_cursor"))?;
    let last: SearchEntitiesCursorLast =
        serde_json::from_value(envelope.last).map_err(|_| Error::from("invalid_cursor"))?;
    let page = execute_search_page(d1, user_id, &filters, Some(&last)).await?;
    Response::from_json(&page)
}

#[instrument(skip_all, fields(action = "search_entities"))]
pub async fn search(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let filters = match parse_search_entities_filters(&req.url()?) {
        Ok(filters) => filters,
        Err(fields) => return validation_error("Invalid query parameters.", fields),
    };
    let d1 = ctx.env.d1("DB")?;
    let page = execute_search_page(&d1, &user_id, &filters, None).await?;
    Response::from_json(&page)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_filters(
        query: &str,
    ) -> std::result::Result<SearchEntitiesFilters, Vec<ValidationFieldError>> {
        let url = Url::parse(&format!("https://example.com/api/search{query}")).unwrap();
        parse_search_entities_filters(&url)
    }

    #[test]
    fn search_filters_require_query() {
        let errors = parse_filters("").expect_err("filters should fail");
        assert_eq!(errors[0].field, "query");
        assert_eq!(errors[0].code, "required");
    }

    #[test]
    fn search_filters_reject_empty_trimmed_query() {
        let errors = parse_filters("?query=%20%20").expect_err("filters should fail");
        assert_eq!(errors[0].field, "query");
        assert_eq!(errors[0].code, "required");
    }

    #[test]
    fn build_match_query_tokenizes_plain_text() {
        let query = build_match_query("milk, bread!").expect("query should build");
        assert_eq!(query, "\"milk\" AND \"bread\"");
    }
}
