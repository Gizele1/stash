use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum BrainError {
    #[error("Maximum active contexts reached (max 4)")]
    MaxContextsReached,

    #[error("Context not found: {0}")]
    ContextNotFound(String),

    #[error("LLM unavailable: {0}")]
    LlmUnavailable(String),

    #[error("Retry queue full")]
    QueueFull,

    #[error("Database error: {0}")]
    DbError(String),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Intent is not compressed")]
    NotCompressed,
}

/// Serializable error for Tauri commands
#[derive(Debug, Serialize)]
pub struct BrainCmdError {
    pub code: String,
    pub message: String,
}

impl From<BrainError> for BrainCmdError {
    fn from(e: BrainError) -> Self {
        let code = match &e {
            BrainError::MaxContextsReached => "MAX_CONTEXTS",
            BrainError::ContextNotFound(_) => "CONTEXT_NOT_FOUND",
            BrainError::LlmUnavailable(_) => "LLM_UNAVAILABLE",
            BrainError::QueueFull => "QUEUE_FULL",
            BrainError::DbError(_) => "DB_ERROR",
            BrainError::InvalidStatus(_) => "INVALID_STATUS",
            BrainError::NotFound(_) => "NOT_FOUND",
            BrainError::NotCompressed => "NOT_COMPRESSED",
        };
        BrainCmdError {
            code: code.to_string(),
            message: e.to_string(),
        }
    }
}

// Allow BrainError to be returned from Tauri commands via String
impl From<BrainError> for String {
    fn from(e: BrainError) -> Self {
        e.to_string()
    }
}
