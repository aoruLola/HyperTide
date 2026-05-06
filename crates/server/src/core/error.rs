use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperTideError {
    Authentication(String),
    PermissionDenied(String),
    Conflict(String),
    NotFound(String),
    Persistence(String),
    Configuration(String),
    Validation(String),
    Internal(String),
}

impl HyperTideError {
    pub fn code(&self) -> &'static str {
        match self {
            HyperTideError::Authentication(_) => "authentication_failed",
            HyperTideError::PermissionDenied(_) => "permission_denied",
            HyperTideError::Conflict(_) => "conflict",
            HyperTideError::NotFound(_) => "not_found",
            HyperTideError::Persistence(_) => "persistence_failed",
            HyperTideError::Configuration(_) => "configuration_error",
            HyperTideError::Validation(_) => "validation_error",
            HyperTideError::Internal(_) => "internal_error",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            HyperTideError::Authentication(message)
            | HyperTideError::PermissionDenied(message)
            | HyperTideError::Conflict(message)
            | HyperTideError::NotFound(message)
            | HyperTideError::Persistence(message)
            | HyperTideError::Configuration(message)
            | HyperTideError::Validation(message)
            | HyperTideError::Internal(message) => message,
        }
    }
}

impl Display for HyperTideError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for HyperTideError {}
