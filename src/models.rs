use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub enum RequestMethod {
    Get,
    Head,
}

impl fmt::Display for RequestMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Head => write!(f, "HEAD"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UrlCheckResult {
    pub url: String,
    pub status: Option<u16>,
    pub time_ms: u128,
    pub redirected: bool,
    pub final_url: String,
    pub error: Option<String>,
    pub attempts: usize,
    pub method: String,
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

impl Summary {
    pub fn has_errors(&self) -> bool {
        self.client_4xx > 0 || self.server_5xx > 0 || self.errors > 0
    }
}
