#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub code: String,
    pub status: u16,
}

impl ApiError {
    pub fn new(code: impl Into<String>, status: u16) -> Self {
        Self {
            code: code.into(),
            status,
        }
    }

    pub fn offline() -> Self {
        Self::new("OFFLINE", 0)
    }

    pub fn is(&self, code: &str) -> bool {
        self.code == code
    }
}
