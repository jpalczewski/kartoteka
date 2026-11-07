//! Public iCal feed: `GET /cal/{token}/feed.ics`.
//!
//! Served outside `/api` and without any session or cookie — the token in the
//! path is the only credential, so it must never reach a log line or a span
//! field. The route is mounted on the base router and rate limited per client
//! IP, because an unauthenticated endpoint is an easy amplification target.
//!
//! The client IP is taken from the peer address unless that peer is a configured
//! trusted reverse proxy (see [`TrustedProxies`]); only then is the
//! `X-Forwarded-For` chain consulted. Client-supplied headers alone never move a
//! request into a fresh bucket.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, Path, Request, State},
    http::{
        HeaderMap, HeaderName, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use ipnet::IpNet;
use kartoteka_db::SqlitePool;
use kartoteka_domain::calendar::{Feed, feed_for_token};
use tower_governor::{
    GovernorError, GovernorLayer, governor::GovernorConfigBuilder, key_extractor::KeyExtractor,
};

use crate::AppState;

/// Requests a single client IP may issue in a burst before throttling.
const BURST_SIZE: u32 = 20;
/// Seconds until one request of the burst quota is replenished.
const REPLENISH_SECONDS: u64 = 3;
/// How often idle per-IP limiter state is dropped.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

const CALENDAR_CONTENT_TYPE: &str = "text/calendar; charset=utf-8";
/// Calendar clients poll on their own schedule; 15 minutes of private caching is plenty.
const CACHE_CONTROL_PRIVATE: &str = "private, max-age=900";
const CONTENT_DISPOSITION_INLINE: &str = "inline; filename=\"kartoteka.ics\"";
const X_CONTENT_TYPE_OPTIONS: &str = "x-content-type-options";
const NOSNIFF: &str = "nosniff";
/// Deliberately information-free bodies: unknown, revoked and malformed tokens
/// must all look the same.
const NOT_FOUND_BODY: &str = "Not Found";
const INTERNAL_ERROR_BODY: &str = "Internal Server Error";

/// Env var listing the reverse proxies whose `X-Forwarded-For` may be believed,
/// as comma-separated CIDRs (`10.0.0.0/8,192.168.1.0/24`) or bare IPs.
const TRUSTED_PROXIES_ENV: &str = "TRUSTED_PROXIES";
const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// Networks whose `X-Forwarded-For` header may be believed.
///
/// Empty means "trust no proxy": every request is keyed on its peer address, so
/// client headers cannot influence rate limiting at all. Operators fill this
/// from the `TRUSTED_PROXIES` env var with the proxy's own addresses (Caddy's
/// container subnet in the compose setup).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrustedProxies(Arc<[IpNet]>);

impl TrustedProxies {
    /// Trust nothing: key every request on its peer address.
    pub fn none() -> Self {
        Self(Arc::from(Vec::new()))
    }

    /// Parse a comma-separated list of CIDRs and/or bare IPs. Bare IPs become a
    /// single-address network (`/32`, `/128`); unparseable entries are dropped
    /// with a warning rather than failing the whole list.
    pub fn parse(raw: &str) -> Self {
        let mut nets = Vec::new();
        for token in raw.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if let Ok(net) = IpNet::from_str(token) {
                if net.prefix_len() == 0 {
                    tracing::warn!(
                        value = %token,
                        "{TRUSTED_PROXIES_ENV} entry with a zero-length prefix trusts every \
                         peer, which lets any client pick its own /cal rate-limit key via \
                         X-Forwarded-For"
                    );
                }
                nets.push(net);
            } else if let Ok(ip) = IpAddr::from_str(token) {
                nets.push(IpNet::from(ip));
            } else {
                tracing::warn!(
                    value = %token,
                    "ignoring invalid {TRUSTED_PROXIES_ENV} entry (expected CIDR or IP)"
                );
            }
        }
        Self(Arc::from(nets))
    }

    /// Read the `TRUSTED_PROXIES` env var. Unset, empty or fully invalid lists
    /// warn once and trust nothing.
    ///
    /// Trusting nothing is the fail-closed default: it cannot be spoofed, but all
    /// clients behind one proxy then share that proxy's rate-limit bucket.
    pub fn from_env() -> Self {
        match std::env::var(TRUSTED_PROXIES_ENV) {
            Ok(raw) => {
                let trusted = Self::parse(&raw);
                if trusted.is_empty() {
                    tracing::warn!(
                        "{TRUSTED_PROXIES_ENV} has no usable entry; trusting no proxy, so all \
                         clients behind a reverse proxy share one /cal rate-limit bucket"
                    );
                }
                trusted
            }
            Err(_) => {
                tracing::warn!(
                    "{TRUSTED_PROXIES_ENV} is unset; trusting no proxy, so clients behind a \
                     reverse proxy share one /cal rate-limit bucket. Set it to the proxy CIDR \
                     (e.g. the Caddy container subnet) in production."
                );
                Self::none()
            }
        }
    }

    /// Whether `ip` is one of the trusted proxies. IPv4-mapped IPv6 addresses
    /// (`::ffff:10.0.0.1`) are compared as IPv4, since that is how dual-stack
    /// sockets report IPv4 peers.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = normalise_ip(ip);
        self.0.iter().any(|net| net.contains(&ip))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Treat `::ffff:a.b.c.d` as the IPv4 address it stands for.
fn normalise_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        IpAddr::V4(_) => ip,
    }
}

/// Parse one `X-Forwarded-For` element: `1.2.3.4`, `1.2.3.4:5678`,
/// `2001:db8::1`, `[2001:db8::1]` or `[2001:db8::1]:5678`. Returns `None` for
/// anything else, including empty elements.
fn parse_ip(raw: &str) -> Option<IpAddr> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(ip) = IpAddr::from_str(raw) {
        return Some(ip);
    }
    // Bracketed IPv6, optionally followed by `:port`.
    if let Some(rest) = raw.strip_prefix('[') {
        let (inner, _) = rest.split_once(']')?;
        return IpAddr::from_str(inner).ok();
    }
    // Bare `v4:port` — exactly one colon, so a bare IPv6 never reaches here.
    let (head, tail) = raw.split_once(':')?;
    if tail.contains(':') {
        return None;
    }
    IpAddr::from_str(head).ok()
}

/// The peer address axum records for the connection, if the server was started
/// with `into_make_service_with_connect_info::<SocketAddr>()`.
fn peer_ip<T>(req: &Request<T>) -> Option<IpAddr> {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0)
        .or_else(|| req.extensions().get::<SocketAddr>().copied())
        .map(|addr| normalise_ip(addr.ip()))
}

/// The client IP from the `X-Forwarded-For` chain, walking **right to left**.
///
/// Append-style proxies add to the right, so entries right of the last trusted
/// proxy are the ones the proxy itself wrote; entries to the left may have been
/// supplied by the client. Trusted hops are skipped, and the first non-trusted
/// parseable entry wins. `X-Real-IP` and `Forwarded` are deliberately not read:
/// Caddy overwrites `X-Forwarded-For` but forwards those two verbatim.
fn forwarded_client_ip(headers: &HeaderMap, trusted: &TrustedProxies) -> Option<IpAddr> {
    let chain: Vec<IpAddr> = headers
        .get_all(X_FORWARDED_FOR)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(parse_ip)
        .map(normalise_ip)
        .collect();
    chain.into_iter().rev().find(|ip| !trusted.contains(*ip))
}

/// Rate-limit key extractor that believes client headers only from trusted peers.
///
/// Unlike tower_governor's stock smart extractor, an untrusted peer's
/// `X-Forwarded-For`, `X-Real-IP` and `Forwarded` headers are never read, so a
/// client cannot rotate fake IPs to escape the limiter. `extract` never fails: a
/// missing peer address maps to one shared bucket ([`Ipv4Addr::UNSPECIFIED`])
/// instead of a 500.
#[derive(Clone, Debug)]
pub struct TrustedProxyIpKeyExtractor {
    trusted: TrustedProxies,
}

impl TrustedProxyIpKeyExtractor {
    pub fn new(trusted: TrustedProxies) -> Self {
        Self { trusted }
    }
}

impl KeyExtractor for TrustedProxyIpKeyExtractor {
    type Key = IpAddr;

    // `name()` and `key_name()` exist only behind tower_governor's `tracing`
    // feature, which this crate disables (default-features = false). Do not add
    // them here — they would not compile.

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let Some(peer) = peer_ip(req) else {
            // No `ConnectInfo`: fail closed into one shared bucket rather than
            // returning `Err`, which tower_governor surfaces as HTTP 500.
            tracing::debug!("no peer address, sharing one rate-limit bucket");
            return Ok(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        };

        if !self.trusted.contains(peer) {
            // Not a trusted proxy: the header is not read at all.
            return Ok(peer);
        }

        match forwarded_client_ip(req.headers(), &self.trusted) {
            Some(client) => Ok(client),
            None => {
                tracing::debug!(%peer, "trusted proxy sent no usable x-forwarded-for; keying on the peer");
                Ok(peer)
            }
        }
    }
}

/// `/cal/{token}/feed.ics` — rate limited per client IP, no auth middleware.
///
/// Burst of [`BURST_SIZE`] requests per IP, then one more every
/// [`REPLENISH_SECONDS`]. The key is the peer address from `ConnectInfo` (which
/// is why `main` serves with `into_make_service_with_connect_info::<SocketAddr>()`),
/// except when that peer is listed in `trusted`: only then is the rightmost
/// non-proxy entry of `X-Forwarded-For` used, so a client cannot dodge the limit
/// by rotating spoofed header values. `X-Real-IP` and `Forwarded` are never read.
pub fn router(trusted: TrustedProxies) -> Router<AppState> {
    let config = GovernorConfigBuilder::default()
        .per_second(REPLENISH_SECONDS)
        .burst_size(BURST_SIZE)
        .key_extractor(TrustedProxyIpKeyExtractor::new(trusted))
        .finish()
        .expect("governor config: burst size and period are non-zero");

    // Drop idle per-IP limiter state so the keyed map cannot grow forever. The
    // task lives on the current runtime, so integration tests get one cheap
    // task per router (cancelled on runtime shutdown) and non-async callers are
    // simply skipped.
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let limiter = config.limiter().clone();
        handle.spawn(async move {
            let mut ticker = tokio::time::interval(CLEANUP_INTERVAL);
            // The first tick of a tokio interval completes immediately.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                limiter.retain_recent();
            }
        });
    }

    Router::new()
        .route("/{token}/feed.ics", get(feed))
        .layer(GovernorLayer::new(config))
}

/// Render the feed for a subscribe token.
///
/// Unknown, revoked and malformed tokens all yield the same bare 404. The token
/// argument is skipped by `skip_all`, so it appears in no span field.
#[tracing::instrument(skip_all, fields(action = "get_calendar_feed"))]
pub async fn feed(
    State(pool): State<SqlitePool>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    match feed_for_token(&pool, &token).await {
        Ok(Some(feed)) => {
            if if_none_match_matches(&headers, &feed.etag) {
                not_modified_response(&feed.etag)
            } else {
                feed_response(feed)
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, NOT_FOUND_BODY).into_response(),
        Err(error) => {
            tracing::error!(%error, "calendar feed generation failed");
            (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_BODY).into_response()
        }
    }
}

/// 200 with the feed body and its caching headers.
fn feed_response(feed: Feed) -> Response {
    let Ok(etag) = HeaderValue::from_str(&feed.etag) else {
        tracing::error!("calendar feed rendered an invalid ETag");
        return (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_BODY).into_response();
    };
    let mut response = Response::new(Body::from(feed.body));
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(CALENDAR_CONTENT_TYPE),
    );
    headers.insert(ETAG, etag);
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL_PRIVATE),
    );
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static(CONTENT_DISPOSITION_INLINE),
    );
    headers.insert(
        HeaderName::from_static(X_CONTENT_TYPE_OPTIONS),
        HeaderValue::from_static(NOSNIFF),
    );
    response
}

/// 304 carrying the validators, so clients keep their cached copy.
fn not_modified_response(etag: &str) -> Response {
    let Ok(etag) = HeaderValue::from_str(etag) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_BODY).into_response();
    };
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    let headers = response.headers_mut();
    headers.insert(ETAG, etag);
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL_PRIVATE),
    );
    response
}

/// `If-None-Match` evaluation: a comma-separated list, `*`, and weak
/// (`W/"…"`) candidates all count as a match for our strong ETag.
fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    let Some(value) = headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let etag = opaque_tag(etag);
    value
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || opaque_tag(candidate) == etag)
}

/// The tag without its optional `W/` prefix and surrounding quotes, so
/// `W/"abc"`, `"abc"` and `abc` all normalise to `abc`.
fn opaque_tag(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = raw.strip_prefix("W/").unwrap_or(raw).trim();
    raw.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_identical_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"abc123\""));
        assert!(if_none_match_matches(&headers, "\"abc123\""));
    }

    #[test]
    fn matches_weak_and_listed_candidates() {
        let mut headers = HeaderMap::new();
        headers.insert(
            IF_NONE_MATCH,
            HeaderValue::from_static("\"other\", W/\"abc123\""),
        );
        assert!(if_none_match_matches(&headers, "\"abc123\""));
    }

    #[test]
    fn matches_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("*"));
        assert!(if_none_match_matches(&headers, "\"abc123\""));
    }

    #[test]
    fn ignores_unrelated_or_missing_headers() {
        let headers = HeaderMap::new();
        assert!(!if_none_match_matches(&headers, "\"abc123\""));

        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"nope\""));
        assert!(!if_none_match_matches(&headers, "\"abc123\""));
    }

    // ---- trust configuration ----

    fn net(raw: &str) -> IpNet {
        IpNet::from_str(raw).expect("valid test network")
    }

    fn ip(raw: &str) -> IpAddr {
        IpAddr::from_str(raw).expect("valid test ip")
    }

    fn addr(raw: &str) -> SocketAddr {
        SocketAddr::from_str(raw).expect("valid test socket address")
    }

    #[test]
    fn parse_accepts_cidrs_and_bare_ips() {
        assert_eq!(
            TrustedProxies::parse("10.0.0.0/8"),
            TrustedProxies(Arc::from(vec![net("10.0.0.0/8")]))
        );
        assert_eq!(
            TrustedProxies::parse("0.0.0.0/0, 192.168.1.7/32"),
            TrustedProxies(Arc::from(vec![net("0.0.0.0/0"), net("192.168.1.7/32")]))
        );
        assert_eq!(
            TrustedProxies::parse("2001:db8::/32,2001:db8::1/128"),
            TrustedProxies(Arc::from(vec![
                net("2001:db8::/32"),
                net("2001:db8::1/128")
            ]))
        );
        // Bare addresses become single-address networks.
        assert_eq!(
            TrustedProxies::parse(" 10.1.2.3 , 2001:db8::5 "),
            TrustedProxies(Arc::from(vec![net("10.1.2.3/32"), net("2001:db8::5/128")]))
        );
    }

    #[test]
    fn parse_drops_junk_but_keeps_valid_entries() {
        let trusted = TrustedProxies::parse("not-a-cidr,10.0.0.0/8,300.1.2.3,,10.0.0.0/33");
        assert_eq!(trusted, TrustedProxies(Arc::from(vec![net("10.0.0.0/8")])));
        assert!(!trusted.is_empty());
    }

    #[test]
    fn parse_of_empty_input_trusts_nothing() {
        for raw in ["", "   ", ","] {
            let trusted = TrustedProxies::parse(raw);
            assert!(trusted.is_empty(), "{raw:?} must trust nothing");
            assert_eq!(trusted, TrustedProxies::none());
        }
    }

    #[test]
    fn contains_matches_v4_and_mapped_v6() {
        let trusted = TrustedProxies::parse("10.0.0.0/8,2001:db8::/32");
        assert!(trusted.contains(ip("10.1.2.3")));
        assert!(!trusted.contains(ip("11.1.2.3")));
        assert!(trusted.contains(ip("2001:db8::1")));
        assert!(!trusted.contains(ip("2001:db9::1")));
        // A dual-stack socket may report an IPv4 peer as `::ffff:a.b.c.d`.
        assert!(trusted.contains(ip("::ffff:10.1.2.3")));
        assert!(!trusted.contains(ip("::ffff:11.1.2.3")));
        assert!(!TrustedProxies::none().contains(ip("10.1.2.3")));
    }

    // ---- X-Forwarded-For selection ----

    fn forwarded(headers: &HeaderMap, trusted: &str) -> Option<IpAddr> {
        forwarded_client_ip(headers, &TrustedProxies::parse(trusted))
    }

    fn headers_with_xff(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            X_FORWARDED_FOR,
            HeaderValue::from_str(value).expect("valid header value"),
        );
        headers
    }

    #[test]
    fn forwarded_single_value() {
        assert_eq!(
            forwarded(&headers_with_xff("198.51.100.10"), "10.0.0.0/8"),
            Some(ip("198.51.100.10"))
        );
    }

    #[test]
    fn forwarded_uses_rightmost_entry_of_a_multi_hop_chain() {
        // Appended by the proxy: client first, proxy hops to the right.
        let headers = headers_with_xff("1.2.3.4, 203.0.113.9, 10.0.0.5");
        assert_eq!(
            forwarded(&headers, "10.0.0.0/8"),
            Some(ip("203.0.113.9")),
            "the entry the proxy appended wins, trusted hops are skipped"
        );
    }

    #[test]
    fn forwarded_ignores_spoofed_leftmost_entry() {
        for spoofed in ["1.2.3.4", "5.6.7.8", "2001:db8:dead::1"] {
            let headers = headers_with_xff(&format!("{spoofed}, 198.51.100.10"));
            assert_eq!(
                forwarded(&headers, "10.0.0.0/8"),
                Some(ip("198.51.100.10")),
                "spoofed leftmost {spoofed} must not become the key"
            );
        }
    }

    #[test]
    fn forwarded_reads_multiple_header_lines_in_wire_order() {
        let mut headers = headers_with_xff("1.2.3.4");
        headers.append(
            X_FORWARDED_FOR,
            HeaderValue::from_static("203.0.113.9, 10.0.0.5"),
        );
        assert_eq!(
            forwarded(&headers, "10.0.0.0/8"),
            Some(ip("203.0.113.9")),
            "the last appended line carries the entry the proxy wrote"
        );
    }

    #[test]
    fn forwarded_chain_of_only_trusted_hops_has_no_client_ip() {
        // Every entry is a proxy, so there is no client address to key on and the
        // caller must fall back to the peer rather than inventing a bucket.
        assert_eq!(
            forwarded(&headers_with_xff("10.0.0.5, 10.0.0.6"), "10.0.0.0/8"),
            None
        );
        assert_eq!(forwarded(&headers_with_xff("10.0.0.5"), "10.0.0.0/8"), None);
    }

    #[test]
    fn forwarded_parses_ports_and_bracketed_ipv6() {
        assert_eq!(
            forwarded(&headers_with_xff("198.51.100.10:51234"), "10.0.0.0/8"),
            Some(ip("198.51.100.10"))
        );
        assert_eq!(
            forwarded(&headers_with_xff("[2001:db8::7]:443"), "10.0.0.0/8"),
            Some(ip("2001:db8::7"))
        );
        assert_eq!(
            forwarded(&headers_with_xff("[2001:db8::7]"), "10.0.0.0/8"),
            Some(ip("2001:db8::7"))
        );
        assert_eq!(
            forwarded(&headers_with_xff("2001:db8::7"), "10.0.0.0/8"),
            Some(ip("2001:db8::7"))
        );
    }

    #[test]
    fn forwarded_without_usable_entry_is_none() {
        for value in [
            "",
            "   ",
            "garbage",
            "1.2.3.4.5",
            "[not-an-ip]:1",
            "1:2:3:4",
        ] {
            assert_eq!(
                forwarded(&headers_with_xff(value), "10.0.0.0/8"),
                None,
                "{value:?} must not yield a key"
            );
        }
        assert_eq!(forwarded(&HeaderMap::new(), "10.0.0.0/8"), None);
    }

    // ---- extractor ----

    fn request_with(peer: Option<&str>, value: Option<&str>) -> Request<()> {
        let mut req = Request::builder().body(()).expect("build request");
        if let Some(peer) = peer {
            req.extensions_mut().insert(ConnectInfo(addr(peer)));
        }
        if let Some(value) = value {
            req.headers_mut().insert(
                X_FORWARDED_FOR,
                HeaderValue::from_str(value).expect("valid header value"),
            );
        }
        req
    }

    fn extract(req: &Request<()>, trusted: &str) -> IpAddr {
        TrustedProxyIpKeyExtractor::new(TrustedProxies::parse(trusted))
            .extract(req)
            .expect("extraction must never fail: an Err is a 500 on /cal")
    }

    #[test]
    fn extract_uses_forwarded_client_for_trusted_peer() {
        let req = request_with(Some("10.1.2.3:5555"), Some("198.51.100.10"));
        assert_eq!(extract(&req, "10.0.0.0/8"), ip("198.51.100.10"));
    }

    #[test]
    fn extract_ignores_spoofed_header_from_untrusted_peer() {
        let req = request_with(Some("203.0.113.7:40000"), Some("198.51.100.10"));
        assert_eq!(
            extract(&req, "10.0.0.0/8"),
            ip("203.0.113.7"),
            "an untrusted peer must be keyed on its own address"
        );
        assert_eq!(
            extract(&req, ""),
            ip("203.0.113.7"),
            "with no trusted proxy, the header is never read"
        );
    }

    #[test]
    fn extract_never_reads_x_real_ip_or_forwarded() {
        let mut req = request_with(Some("10.1.2.3:5555"), None);
        req.headers_mut()
            .insert("x-real-ip", HeaderValue::from_static("1.2.3.4"));
        req.headers_mut()
            .insert("forwarded", HeaderValue::from_static("for=5.6.7.8"));
        assert_eq!(
            extract(&req, "10.0.0.0/8"),
            ip("10.1.2.3"),
            "only X-Forwarded-For is trusted, and only when present"
        );
    }

    #[test]
    fn extract_falls_back_to_peer_when_forwarded_is_unusable() {
        let req = request_with(Some("10.1.2.3:5555"), Some("garbage"));
        assert_eq!(extract(&req, "10.0.0.0/8"), ip("10.1.2.3"));
    }

    #[test]
    fn extract_without_peer_returns_shared_sentinel() {
        let req = request_with(None, Some("198.51.100.10"));
        assert_eq!(
            extract(&req, "10.0.0.0/8"),
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            "a missing peer must share one bucket, not 500"
        );
    }

    #[test]
    fn extract_accepts_socket_addr_extension_too() {
        let mut req = Request::builder().body(()).expect("build request");
        req.extensions_mut().insert(addr("203.0.113.9:1234"));
        assert_eq!(extract(&req, ""), ip("203.0.113.9"));
    }
}
