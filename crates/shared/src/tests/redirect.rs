use crate::redirect::safe_return_to;

#[test]
fn allows_same_origin_paths() {
    assert_eq!(safe_return_to("/"), "/");
    assert_eq!(safe_return_to("/lists/abc?x=1"), "/lists/abc?x=1");
    assert_eq!(safe_return_to("/today#x"), "/today#x");
}

#[test]
fn rejects_protocol_relative_and_absolute_urls() {
    assert_eq!(safe_return_to("//evil.com"), "/");
    assert_eq!(safe_return_to("/\\evil.com"), "/");
    assert_eq!(safe_return_to("https://evil.com"), "/");
    assert_eq!(safe_return_to("javascript:alert(1)"), "/");
    assert_eq!(safe_return_to("evil.com"), "/");
    assert_eq!(safe_return_to(""), "/");
}

#[test]
fn rejects_control_chars_that_browsers_strip() {
    assert_eq!(safe_return_to("/\t/evil.com"), "/");
    assert_eq!(safe_return_to("/\n/evil.com"), "/");
    assert_eq!(safe_return_to("/\r/evil.com"), "/");
    assert_eq!(safe_return_to("/\u{0}/evil.com"), "/");
}

#[test]
fn rejects_backslash_anywhere() {
    assert_eq!(safe_return_to("/a\\b"), "/");
    assert_eq!(safe_return_to("/a/b\\"), "/");
}
