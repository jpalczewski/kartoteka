use kartoteka_shared::{ErrorResponse, ValidationFieldError};
use worker::Response;

pub fn json_error(code: &str, status: u16) -> worker::Result<Response> {
    let body = ErrorResponse {
        code: Some(code.to_string()),
        status,
        message: None,
        fields: vec![],
    };
    Response::from_json(&body).map(|r| r.with_status(status))
}

pub fn validation_error(
    message: &str,
    fields: Vec<ValidationFieldError>,
) -> worker::Result<Response> {
    let body = ErrorResponse {
        code: Some("validation_failed".to_string()),
        status: 422,
        message: Some(message.to_string()),
        fields,
    };
    Response::from_json(&body).map(|r| r.with_status(422))
}
