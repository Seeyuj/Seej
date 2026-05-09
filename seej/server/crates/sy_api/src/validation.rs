//! # Validation
//!
//! Input sanitization and compatibility/versioning logic.

use crate::commands::{CreateWorldCmd, CreateZoneCmd, SimCommand, SpawnEntityCmd};
use crate::errors::ValidationError;
use sy_types::{SimError, SimResult};

/// Validate a command accepted by the pure simulation core.
pub fn validate_sim_command(cmd: &SimCommand) -> Result<(), Vec<ValidationError>> {
    let errors = match cmd {
        SimCommand::CreateWorld(c) => validate_create_world(c),
        SimCommand::SpawnEntity(c) => validate_spawn_entity(c),
        SimCommand::CreateZone(c) => validate_create_zone(c),
        SimCommand::TickN(n) => {
            if *n == 0 {
                vec![ValidationError::new("n", "Tick count must be > 0")]
            } else if *n > 10000 {
                vec![ValidationError::new(
                    "n",
                    "Tick count too large (max 10000)",
                )]
            } else {
                vec![]
            }
        }
        _ => vec![],
    };

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a persisted world identifier before filesystem path construction.
pub fn validate_world_id(world_id: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if world_id.is_empty() {
        errors.push(ValidationError::new("world_id", "World ID cannot be empty"));
        return errors;
    }

    if world_id == "." || world_id == ".." || world_id.contains("..") {
        errors.push(ValidationError::new(
            "world_id",
            "World ID cannot contain path traversal",
        ));
    }

    if world_id.contains('/') || world_id.contains('\\') || world_id.contains('\0') {
        errors.push(ValidationError::new(
            "world_id",
            "World ID cannot contain path separators or NUL",
        ));
    }

    if world_id.contains(':') {
        errors.push(ValidationError::new(
            "world_id",
            "World ID cannot contain drive or scheme separators",
        ));
    }

    if !world_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        errors.push(ValidationError::new(
            "world_id",
            "World ID must use only ASCII letters, numbers, '_' or '-'",
        ));
    }

    errors
}

/// Validate a persisted world identifier and return a simulation error on failure.
pub fn validate_world_id_result(world_id: &str) -> SimResult<()> {
    let errors = validate_world_id(world_id);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(SimError::InvalidOperation(format!(
            "Invalid world_id '{}': {:?}",
            world_id, errors
        )))
    }
}

fn validate_create_world(cmd: &CreateWorldCmd) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if cmd.name.is_empty() {
        errors.push(ValidationError::new("name", "World name cannot be empty"));
    }
    if cmd.name.len() > 64 {
        errors.push(ValidationError::new(
            "name",
            "World name too long (max 64 chars)",
        ));
    }

    errors
}

fn validate_spawn_entity(_cmd: &SpawnEntityCmd) -> Vec<ValidationError> {
    // Basic validation - can be extended later
    Vec::new()
}

fn validate_create_zone(cmd: &CreateZoneCmd) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(name) = &cmd.name {
        if name.len() > 64 {
            errors.push(ValidationError::new(
                "name",
                "Zone name too long (max 64 chars)",
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use sy_types::RngSeed;

    #[test]
    fn validate_empty_world_name() {
        let cmd = SimCommand::CreateWorld(CreateWorldCmd {
            name: String::new(),
            seed: RngSeed::new(42),
        });
        let result = validate_sim_command(&cmd);
        assert!(result.is_err());
    }

    #[test]
    fn validate_tick_zero() {
        let result = validate_sim_command(&SimCommand::TickN(0));
        assert!(result.is_err());
    }

    #[test]
    fn validate_world_id_rejects_path_input() {
        for id in [
            "..",
            "../world",
            r"..\world",
            "/abs",
            r"C:\world",
            "world/1",
        ] {
            assert!(!validate_world_id(id).is_empty(), "{id} must be rejected");
        }
    }

    #[test]
    fn validate_world_id_accepts_storage_key() {
        assert!(validate_world_id("world_42").is_empty());
        assert!(validate_world_id("world-42_A").is_empty());
    }
}
