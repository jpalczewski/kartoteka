use crate::McpI18n;
use rmcp::model::{Annotated, RawResource, RawResourceTemplate};

// Re-export types the server needs
pub use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult, ResourceContents,
};

#[derive(Debug, PartialEq)]
pub enum ResourceUri {
    Lists,
    ListDetail(String),
    ListItems {
        list_id: String,
        cursor: Option<String>,
        limit: Option<u32>,
    },
    Containers,
    ContainerDetail(String),
    Tags {
        cursor: Option<String>,
    },
    Today,
    TimeSummary,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("bad scheme")]
    BadScheme,
    #[error("unknown path")]
    UnknownPath,
}

pub fn parse(uri: &str) -> Result<ResourceUri, ParseError> {
    let rest = uri
        .strip_prefix("kartoteka://")
        .ok_or(ParseError::BadScheme)?;
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let params: std::collections::HashMap<String, String> = query
        .map(|q| {
            q.split('&')
                .filter_map(|kv| {
                    kv.split_once('=').map(|(k, v)| {
                        (
                            k.to_string(),
                            urlencoding::decode(v)
                                .ok()
                                .map(|c| c.into_owned())
                                .unwrap_or_default(),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    Ok(match segs.as_slice() {
        ["lists"] => ResourceUri::Lists,
        ["lists", id] => ResourceUri::ListDetail((*id).into()),
        ["lists", id, "items"] => ResourceUri::ListItems {
            list_id: (*id).into(),
            cursor: params.get("cursor").cloned(),
            limit: params.get("limit").and_then(|s| s.parse().ok()),
        },
        ["containers"] => ResourceUri::Containers,
        ["containers", id] => ResourceUri::ContainerDetail((*id).into()),
        ["tags"] => ResourceUri::Tags {
            cursor: params.get("cursor").cloned(),
        },
        ["today"] => ResourceUri::Today,
        ["time", "summary"] => ResourceUri::TimeSummary,
        _ => return Err(ParseError::UnknownPath),
    })
}

pub fn static_resources(i18n: &McpI18n, locale: &str) -> Vec<rmcp::model::Resource> {
    let mk = |uri: &str, name: &str, key: &str| {
        Annotated::new(
            RawResource {
                uri: uri.into(),
                name: name.into(),
                description: Some(i18n.translate(locale, key)),
                mime_type: Some("application/json".into()),
                size: None,
            },
            None,
        )
    };
    vec![
        mk("kartoteka://lists", "lists", "mcp-res-lists-desc"),
        mk(
            "kartoteka://containers",
            "containers",
            "mcp-res-containers-desc",
        ),
        mk("kartoteka://today", "today", "mcp-res-today-desc"),
        mk(
            "kartoteka://time/summary",
            "time_summary",
            "mcp-res-time-summary-desc",
        ),
    ]
}

pub fn resource_templates(i18n: &McpI18n, locale: &str) -> Vec<rmcp::model::ResourceTemplate> {
    let mk = |uri: &str, name: &str, key: &str| {
        Annotated::new(
            RawResourceTemplate {
                uri_template: uri.into(),
                name: name.into(),
                description: Some(i18n.translate(locale, key)),
                mime_type: Some("application/json".into()),
            },
            None,
        )
    };
    vec![
        mk(
            "kartoteka://lists/{list_id}",
            "list_detail",
            "mcp-res-list-detail-desc",
        ),
        mk(
            "kartoteka://lists/{list_id}/items{?cursor,limit}",
            "list_items",
            "mcp-res-list-items-desc",
        ),
        mk(
            "kartoteka://containers/{container_id}",
            "container_detail",
            "mcp-res-container-detail-desc",
        ),
        mk("kartoteka://tags{?cursor}", "tags", "mcp-res-tags-desc"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_uris() {
        assert_eq!(parse("kartoteka://lists").unwrap(), ResourceUri::Lists);
        assert_eq!(
            parse("kartoteka://containers").unwrap(),
            ResourceUri::Containers
        );
        assert_eq!(parse("kartoteka://today").unwrap(), ResourceUri::Today);
        assert_eq!(
            parse("kartoteka://time/summary").unwrap(),
            ResourceUri::TimeSummary
        );
    }

    #[test]
    fn dynamic_uris() {
        assert_eq!(
            parse("kartoteka://lists/abc").unwrap(),
            ResourceUri::ListDetail("abc".into())
        );
        assert_eq!(
            parse("kartoteka://containers/xyz").unwrap(),
            ResourceUri::ContainerDetail("xyz".into())
        );
    }

    #[test]
    fn items_with_cursor_limit() {
        let r = parse("kartoteka://lists/abc/items?cursor=eyJ&limit=25").unwrap();
        match r {
            ResourceUri::ListItems {
                list_id,
                cursor,
                limit,
            } => {
                assert_eq!(list_id, "abc");
                assert_eq!(cursor.as_deref(), Some("eyJ"));
                assert_eq!(limit, Some(25));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn rejects_bad_scheme() {
        assert!(parse("https://evil.com/foo").is_err());
    }

    #[test]
    fn rejects_unknown_path() {
        assert!(parse("kartoteka://nothing").is_err());
        assert!(parse("kartoteka://lists/abc/badsegment").is_err());
    }
}
