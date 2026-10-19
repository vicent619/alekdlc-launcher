#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub code: String,
    pub status: u16,
    pub detail: Option<String>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, status: u16) -> Self {
        Self {
            code: code.into(),
            status,
            detail: None,
        }
    }

    pub fn offline() -> Self {
        Self::new(crate::secret!("OFFLINE"), 0)
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn is(&self, code: &str) -> bool {
        self.code == code
    }
}
