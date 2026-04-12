use kartoteka_shared::{ErrorResponse, ValidationFieldError};
use worker::Response;

fn fallback_message_for_code(code: &str) -> String {
    let normalized = code.replace('_', " ");
    let mut chars = normalized.chars();
    let Some(first) = chars.next() else {
        return "Request failed.".to_string();
    };
    let mut message = first.to_uppercase().collect::<String>();
    message.push_str(chars.as_str());
    if !message.ends_with('.') {
        message.push('.');
    }
    message
}

fn default_message_for_code(code: &str) -> String {
    match code {
        "validation_failed" => "Validation failed.".to_string(),
        "invalid_request_body" => "Invalid request body.".to_string(),
        "invalid_config" => "Invalid configuration.".to_string(),
        "invalid_locale" => "Invalid locale.".to_string(),
        "list_not_found" => "List not found.".to_string(),
        "item_not_found" => "Item not found.".to_string(),
        "container_not_found" => "Container not found.".to_string(),
        "tag_not_found" => "Tag not found.".to_string(),
        "list_archived" => "List is archived.".to_string(),
        "feature_required" => "Feature is not enabled.".to_string(),
        "tag_self_parent" => "Tag cannot be its own parent.".to_string(),
        "tag_cycle" => "Tag hierarchy would create a cycle.".to_string(),
        "invalid_container_hierarchy" => "Invalid container hierarchy.".to_string(),
        "invalid_container_move" => "Invalid container move.".to_string(),
        "invalid_cursor" => "Invalid cursor.".to_string(),
        "unauthorized" => "Unauthorized.".to_string(),
        _ => fallback_message_for_code(code),
    }
}

pub fn json_error(code: &str, status: u16) -> worker::Result<Response> {
    let body = ErrorResponse {
        code: Some(code.to_string()),
        status,
        message: Some(default_message_for_code(code)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_code_gets_human_message() {
        assert_eq!(
            default_message_for_code("list_not_found"),
            "List not found."
        );
    }

    #[test]
    fn unknown_code_is_humanized() {
        assert_eq!(default_message_for_code("some_new_code"), "Some new code.");
    }
}
