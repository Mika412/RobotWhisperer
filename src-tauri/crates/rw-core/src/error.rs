use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("schema error: {0}")]
    Schema(String),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_format_includes_message() {
        let error = CoreError::Storage("disk full".into());
        assert_eq!(error.to_string(), "storage error: disk full");

        let error = CoreError::NotFound("request 42".into());
        assert_eq!(error.to_string(), "not found: request 42");
    }

    #[test]
    fn from_io_error_preserves_message() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let core_error: CoreError = io_error.into();
        assert!(matches!(core_error, CoreError::Io(_)));
        assert!(core_error.to_string().contains("missing file"));
    }

    #[test]
    fn from_serde_error_preserves_message() {
        let serde_error = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let core_error: CoreError = serde_error.into();
        assert!(matches!(core_error, CoreError::Serde(_)));
    }
}
