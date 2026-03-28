use serde::Serialize;
use worker::Response;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub status: u16,
}

pub fn json_error(code: &str, status: u16) -> worker::Result<Response> {
    let body = ErrorResponse {
        code: code.to_string(),
        status,
    };
    Response::from_json(&body).map(|r| r.with_status(status))
}
