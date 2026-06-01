use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UrlCheckResult {
    pub url: String,
    pub status: Option<u16>,
    pub time_ms: u128,
    pub redirected: bool,
    pub final_url: String,
    pub error: Option<String>,
    pub attempts: usize,
}

impl UrlCheckResult {
    pub fn is_error(&self) -> bool {
        self.error.is_some() || self.status.map(|s| s >= 400).unwrap_or(true)
    }
}

#[derive(Debug, Default)]
pub struct Summary {
    pub total: usize,
    pub ok_2xx: usize,
    pub redirect_3xx: usize,
    pub client_4xx: usize,
    pub server_5xx: usize,
    pub errors: usize,
    pub average_time_ms: u128,
    pub slowest: Vec<UrlCheckResult>,
}
