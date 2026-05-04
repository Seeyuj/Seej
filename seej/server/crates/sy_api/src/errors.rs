//! # Errors
//!
//! Typed errors for validation, refusal, and compatibility issues.

use serde::{Deserialize, Serialize};
use sy_types::{EntityId, ZoneId};

/// API-level errors (command processing failures)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    /// World not loaded
    NoWorldLoaded,
    /// World already exists
    WorldAlreadyExists(String),
    /// World not found
    WorldNotFound(String),
    /// Entity not found
    EntityNotFound(EntityId),
    /// Zone not found
    ZoneNotFound(ZoneId),
    /// Zone already exists
    ZoneAlreadyExists(ZoneId),
    /// Invalid command
    InvalidCommand(String),
    /// Validation failed
    ValidationFailed(Vec<ValidationError>),
    /// Storage error
    StorageError(String),
    /// Internal error
    InternalError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NoWorldLoaded => write!(f, "No world is currently loaded"),
            ApiError::WorldAlreadyExists(id) => write!(f, "World already exists: {}", id),
            ApiError::WorldNotFound(id) => write!(f, "World not found: {}", id),
            ApiError::EntityNotFound(id) => write!(f, "Entity not found: {}", id),
            ApiError::ZoneNotFound(id) => write!(f, "Zone not found: {}", id),
            ApiError::ZoneAlreadyExists(id) => write!(f, "Zone already exists: {}", id),
            ApiError::InvalidCommand(msg) => write!(f, "Invalid command: {}", msg),
            ApiError::ValidationFailed(errors) => {
                write!(f, "Validation failed: {:?}", errors)
            }
            ApiError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            ApiError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;
