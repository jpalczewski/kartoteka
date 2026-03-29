use std::future::Future;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub trait HttpClient {
    fn request(
        &self,
        method: Method,
        url: &str,
        body: Option<&str>,
    ) -> impl Future<Output = Result<HttpResponse, String>>;
}

/// Production HTTP client using gloo-net. Sends credentials with every request.
/// On non-wasm targets the impl panics — only ever instantiated in WASM context.
#[derive(Clone)]
pub struct GlooClient;

#[cfg(target_arch = "wasm32")]
impl HttpClient for GlooClient {
    async fn request(
        &self,
        method: Method,
        url: &str,
        body: Option<&str>,
    ) -> Result<HttpResponse, String> {
        use gloo_net::http::Request;

        let headers = super::auth_headers();

        let builder = match method {
            Method::Get => Request::get(url),
            Method::Post => Request::post(url),
            Method::Put => Request::put(url),
            Method::Patch => Request::patch(url),
            Method::Delete => Request::delete(url),
        };

        let builder = builder
            .headers(headers)
            .credentials(web_sys::RequestCredentials::Include);

        let resp = if let Some(b) = body {
            builder.body(b).map_err(|e| e.to_string())?.send().await
        } else {
            builder.send().await
        }
        .map_err(|e| e.to_string())?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;

        Ok(HttpResponse { status, body: text })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl HttpClient for GlooClient {
    async fn request(
        &self,
        _method: Method,
        _url: &str,
        _body: Option<&str>,
    ) -> Result<HttpResponse, String> {
        unimplemented!("GlooClient is a WASM-only HTTP client")
    }
}
