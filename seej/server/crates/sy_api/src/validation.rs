//! # Validation
//!
//! Input sanitization and compatibility/versioning logic.

use crate::commands::{Command, CreateWorldCmd, CreateZoneCmd, SpawnEntityCmd};
use crate::errors::ValidationError;

/// Validate a command before processing
pub fn validate_command(cmd: &Command) -> Result<(), Vec<ValidationError>> {
    let errors = match cmd {
        Command::CreateWorld(c) => validate_create_world(c),
        Command::SpawnEntity(c) => validate_spawn_entity(c),
        Command::CreateZone(c) => validate_create_zone(c),
        Command::TickN(n) => {
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
        // Other commands don't need validation for now
        _ => vec![],
    };

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
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
        let cmd = Command::CreateWorld(CreateWorldCmd {
            name: String::new(),
            seed: RngSeed::new(42),
        });
        let result = validate_command(&cmd);
        assert!(result.is_err());
    }

    #[test]
    fn validate_tick_zero() {
        let result = validate_command(&Command::TickN(0));
        assert!(result.is_err());
    }
}
