use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Erro de rede: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Erro de arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Erro de ZIP: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn msg(m: impl Into<String>) -> Self {
        AppError::Other(m.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
