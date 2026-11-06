//! Validation of user-supplied redirect targets (`return_to`), shared by the
//! client and every server-side redirect path so they accept exactly the same
//! values.

/// Return `raw` if it is a safe same-origin path, otherwise `"/"`.
///
/// A value is accepted only when it:
/// - starts with exactly one `/`,
/// - does not start with a second `/` or a `\` (protocol-relative URLs),
/// - contains no `\`, no `:` and no control characters.
///
/// Browsers strip tab/CR/LF from URLs before parsing, so `/\t/evil.com` would
/// otherwise be interpreted as `//evil.com` and navigate off-site.
pub fn safe_return_to(raw: &str) -> &str {
    let mut chars = raw.chars();
    if chars.next() != Some('/') {
        return "/";
    }
    if matches!(chars.next(), Some('/' | '\\')) {
        return "/";
    }
    if raw.chars().any(|c| c == '\\' || c == ':' || c.is_control()) {
        return "/";
    }
    raw
}
