use kartoteka_shared::ErrorResponse;
use worker::Response;

pub fn json_error(code: &str, status: u16) -> worker::Result<Response> {
    let body = ErrorResponse {
        code: Some(code.to_string()),
        status,
    };
    Response::from_json(&body).map(|r| r.with_status(status))
}
