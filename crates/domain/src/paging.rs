use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Paged<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub limit: u32,
}

pub fn clamp_limit(opt: Option<u32>) -> u32 {
    opt.unwrap_or(100).clamp(1, 500)
}
