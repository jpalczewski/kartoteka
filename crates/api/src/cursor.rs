use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

const CURSOR_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageCursorEnvelope {
    pub v: u8,
    pub kind: String,
    pub limit: u32,
    pub params: serde_json::Value,
    pub last: serde_json::Value,
}

pub fn encode_cursor<TParams: Serialize, TLast: Serialize>(
    kind: &str,
    limit: u32,
    params: &TParams,
    last: &TLast,
) -> Result<String, serde_json::Error> {
    let envelope = PageCursorEnvelope {
        v: CURSOR_VERSION,
        kind: kind.to_string(),
        limit,
        params: serde_json::to_value(params)?,
        last: serde_json::to_value(last)?,
    };
    let bytes = serde_json::to_vec(&envelope)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn decode_cursor(raw: &str) -> Result<PageCursorEnvelope, &'static str> {
    let bytes = URL_SAFE_NO_PAD.decode(raw).map_err(|_| "invalid_cursor")?;
    let envelope: PageCursorEnvelope =
        serde_json::from_slice(&bytes).map_err(|_| "invalid_cursor")?;
    if envelope.v != CURSOR_VERSION {
        return Err("invalid_cursor");
    }
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trip() {
        let raw = encode_cursor(
            "lists",
            10,
            &serde_json::json!({ "archived": false }),
            &serde_json::json!({ "id": "l1" }),
        )
        .expect("cursor should encode");
        let decoded = decode_cursor(&raw).expect("cursor should decode");

        assert_eq!(decoded.kind, "lists");
        assert_eq!(decoded.limit, 10);
        assert_eq!(decoded.params["archived"], false);
        assert_eq!(decoded.last["id"], "l1");
    }

    #[test]
    fn invalid_cursor_rejected() {
        assert_eq!(decode_cursor("not-a-valid-cursor"), Err("invalid_cursor"));
    }
}
