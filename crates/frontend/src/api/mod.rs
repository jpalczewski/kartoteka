pub mod admin;
pub mod client;
mod containers;
mod items;
mod lists;
pub mod preferences;
mod search;
mod settings;
mod tags;

pub use containers::*;
pub use items::*;
pub use lists::*;
pub use search::*;
pub use settings::*;
pub use tags::*;

pub(crate) use client::{HttpClient, HttpResponse, Method};

pub(crate) fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

/// Structured API error type for i18n-aware error display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ApiError {
    Network(String),
    Http { status: u16, code: Option<String> },
    Parse(String),
}

impl ApiError {
    #[allow(dead_code)]
    pub fn to_i18n_key(&self) -> &'static str {
        match self {
            ApiError::Network(_) => "error-network",
            ApiError::Http { code, .. } => match code.as_deref() {
                Some("list_not_found") => "error-list-not-found",
                Some("item_not_found") => "error-item-not-found",
                Some("container_not_found") => "error-container-not-found",
                Some("tag_not_found") => "error-tag-not-found",
                Some("unauthorized") => "error-unauthorized",
                _ => "error-http",
            },
            ApiError::Parse(_) => "error-unknown",
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Network(e) => write!(f, "Network error: {e}"),
            ApiError::Http { status, code } => {
                if let Some(c) = code {
                    write!(f, "HTTP {status}: {c}")
                } else {
                    write!(f, "HTTP error {status}")
                }
            }
            ApiError::Parse(e) => write!(f, "Parse error: {e}"),
        }
    }
}

pub(crate) const API_BASE: &str = match option_env!("API_BASE_URL") {
    Some(v) => v,
    None => "/api",
};

/// Build auth headers (Content-Type: application/json).
/// Only available on wasm32.
#[cfg(target_arch = "wasm32")]
pub(crate) fn auth_headers() -> gloo_net::http::Headers {
    let headers = gloo_net::http::Headers::new();
    headers.set("Content-Type", "application/json");
    headers
}

/// Parse response body, checking HTTP status code first.
pub(crate) fn parse_response<T: serde::de::DeserializeOwned>(
    resp: &HttpResponse,
) -> Result<T, ApiError> {
    if resp.status >= 400 {
        let code = serde_json::from_str::<kartoteka_shared::ErrorResponse>(&resp.body)
            .ok()
            .and_then(|e| e.code);
        return Err(ApiError::Http {
            status: resp.status,
            code,
        });
    }
    serde_json::from_str(&resp.body).map_err(|e| ApiError::Parse(e.to_string()))
}

/// Parse response with no body (204 No Content). Returns Ok if status < 400.
pub(crate) fn parse_empty_response(resp: &HttpResponse) -> Result<(), ApiError> {
    if resp.status >= 400 {
        let code = serde_json::from_str::<kartoteka_shared::ErrorResponse>(&resp.body)
            .ok()
            .and_then(|e| e.code);
        return Err(ApiError::Http {
            status: resp.status,
            code,
        });
    }
    Ok(())
}

pub(crate) async fn api_get<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    url: &str,
) -> Result<T, ApiError> {
    let resp = client
        .request(Method::Get, url, None)
        .await
        .map_err(ApiError::Network)?;
    parse_response(&resp)
}

pub(crate) async fn fetch_next_page<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    cursor: &str,
) -> Result<kartoteka_shared::CursorPage<T>, ApiError> {
    let encoded = encode_query_component(cursor);
    api_get(client, &format!("{}/next-page?cursor={encoded}", API_BASE)).await
}

pub(crate) async fn collect_all_pages<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    mut page: kartoteka_shared::CursorPage<T>,
) -> Result<Vec<T>, ApiError> {
    let mut items = page.items;
    let mut cursor = page.next_cursor;

    while let Some(next_cursor) = cursor {
        page = fetch_next_page(client, &next_cursor).await?;
        items.extend(page.items);
        cursor = page.next_cursor;
    }

    Ok(items)
}

pub(crate) async fn api_post<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let resp = client
        .request(Method::Post, url, Some(&json))
        .await
        .map_err(ApiError::Network)?;
    parse_response(&resp)
}

pub(crate) async fn api_post_empty(
    client: &impl HttpClient,
    url: &str,
    body: &impl serde::Serialize,
) -> Result<(), ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let resp = client
        .request(Method::Post, url, Some(&json))
        .await
        .map_err(ApiError::Network)?;
    parse_empty_response(&resp)
}

pub(crate) async fn api_put<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let resp = client
        .request(Method::Put, url, Some(&json))
        .await
        .map_err(ApiError::Network)?;
    parse_response(&resp)
}

pub(crate) async fn api_put_empty(
    client: &impl HttpClient,
    url: &str,
    body: &impl serde::Serialize,
) -> Result<(), ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let resp = client
        .request(Method::Put, url, Some(&json))
        .await
        .map_err(ApiError::Network)?;
    parse_empty_response(&resp)
}

pub(crate) async fn api_patch<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let resp = client
        .request(Method::Patch, url, Some(&json))
        .await
        .map_err(ApiError::Network)?;
    parse_response(&resp)
}

pub(crate) async fn api_patch_empty(
    client: &impl HttpClient,
    url: &str,
    body: &impl serde::Serialize,
) -> Result<(), ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let resp = client
        .request(Method::Patch, url, Some(&json))
        .await
        .map_err(ApiError::Network)?;
    parse_empty_response(&resp)
}

pub(crate) async fn api_delete(client: &impl HttpClient, url: &str) -> Result<(), ApiError> {
    let resp = client
        .request(Method::Delete, url, None)
        .await
        .map_err(ApiError::Network)?;
    parse_empty_response(&resp)
}

/// Auth base URL — derived from API_BASE_URL.
/// Locally API_BASE_URL="/api" so Trunk proxy handles /auth/* via window origin.
/// In prod/dev API_BASE_URL="https://gateway.../api" so strip "/api" to get gateway root.
pub fn auth_base() -> String {
    if API_BASE.starts_with("http") {
        API_BASE.trim_end_matches("/api").to_string()
    } else {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|w| w.location().origin().ok())
                .unwrap_or_default()
        }
        #[cfg(not(target_arch = "wasm32"))]
        String::new()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SessionInfo {
    pub user: SessionUser,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
pub struct SessionUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

/// Check current session. Returns Some(SessionInfo) if logged in.
pub async fn get_session() -> Option<SessionInfo> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;
        let url = format!("{}/auth/api/get-session", auth_base());
        let resp = Request::get(&url)
            .credentials(web_sys::RequestCredentials::Include)
            .send()
            .await
            .ok()?;
        if resp.status() == 200 {
            resp.json::<SessionInfo>().await.ok()
        } else {
            None
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// Sign out and redirect to /login
pub fn logout() {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;
        leptos::task::spawn_local(async {
            let url = format!("{}/auth/api/sign-out", auth_base());
            let _ = Request::post(&url)
                .credentials(web_sys::RequestCredentials::Include)
                .send()
                .await;
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href("/login");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::client::{HttpClient, HttpResponse, Method};
    use super::*;

    struct MockClient {
        status: u16,
        body: String,
    }

    impl MockClient {
        fn ok(body: &str) -> Self {
            Self {
                status: 200,
                body: body.to_string(),
            }
        }
        fn error(status: u16, code: &str) -> Self {
            Self {
                status,
                body: serde_json::json!({"code": code, "status": status}).to_string(),
            }
        }
        fn no_content() -> Self {
            Self {
                status: 204,
                body: String::new(),
            }
        }
    }

    impl HttpClient for MockClient {
        async fn request(
            &self,
            _method: Method,
            _url: &str,
            _body: Option<&str>,
        ) -> Result<HttpResponse, String> {
            Ok(HttpResponse {
                status: self.status,
                body: self.body.clone(),
            })
        }
    }

    #[tokio::test]
    async fn test_parse_response_success() {
        let resp = HttpResponse {
            status: 200,
            body: r#"[1, 2, 3]"#.to_string(),
        };
        let result: Result<Vec<i32>, ApiError> = parse_response(&resp);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_parse_response_http_error() {
        let resp = HttpResponse {
            status: 404,
            body: r#"{"code": "not_found", "status": 404}"#.to_string(),
        };
        let result: Result<Vec<i32>, ApiError> = parse_response(&resp);
        assert!(matches!(result, Err(ApiError::Http { status: 404, .. })));
        if let Err(ApiError::Http { code, .. }) = result {
            assert_eq!(code.as_deref(), Some("not_found"));
        }
    }

    #[tokio::test]
    async fn test_api_get_success() {
        use kartoteka_shared::List;
        let list_json = r#"[{"id":"1","user_id":"u1","name":"Test","list_type":"checklist","position":0,"archived":0,"features":[],"created_at":"2026-01-01","updated_at":"2026-01-01"}]"#;
        let client = MockClient::ok(list_json);
        let result: Result<Vec<List>, ApiError> = api_get(&client, "/test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_delete_no_content() {
        let client = MockClient::no_content();
        let result = api_delete(&client, "/test/123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_delete_error() {
        let client = MockClient::error(404, "not_found");
        let result = api_delete(&client, "/test/123").await;
        assert!(matches!(result, Err(ApiError::Http { status: 404, .. })));
    }
}
