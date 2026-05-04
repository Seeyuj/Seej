//! # Scenarios
//!
//! World builders and test case definitions.
//!
//! ## Usage
//! ```ignore
//! let sim = TestScenario::empty_world(42).build();
//! ```

use sy_api::commands::{Command, CreateWorldCmd, EntityProperties, SpawnEntityCmd};
use sy_core::Simulation;
use sy_types::{EntityKind, Position, RngSeed, WorldPos, ZoneId};

use crate::mocks::{MockClock, MockEventLog, MockRng, MockWorldStore};

/// Builder for test scenarios.
pub struct TestScenario {
    seed: RngSeed,
    world_name: String,
    entities_to_spawn: Vec<SpawnEntityCmd>,
}

impl TestScenario {
    /// Create an empty world scenario.
    pub fn empty_world(seed: u64) -> Self {
        TestScenario {
            seed: RngSeed::new(seed),
            world_name: "Test World".to_string(),
            entities_to_spawn: Vec::new(),
        }
    }

    /// Set the world name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.world_name = name.into();
        self
    }

    /// Add a resource entity.
    pub fn with_resource(mut self, x: i32, y: i32, amount: u32) -> Self {
        self.entities_to_spawn.push(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new(x, y, 0)),
            kind: EntityKind::Resource,
            properties: EntityProperties {
                name: Some("Resource".to_string()),
                amount: Some(amount),
                health: None,
            },
        });
        self
    }

    /// Add a creature entity.
    pub fn with_creature(mut self, x: i32, y: i32, health: u32) -> Self {
        self.entities_to_spawn.push(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new(x, y, 0)),
            kind: EntityKind::Creature,
            properties: EntityProperties {
                name: Some("Creature".to_string()),
                amount: None,
                health: Some(health),
            },
        });
        self
    }

    /// Build the simulation and create the world.
    pub fn build(self) -> Simulation<MockRng, MockClock, MockEventLog, MockWorldStore> {
        let rng = MockRng::new(self.seed);
        let clock = MockClock::new();
        let event_log = MockEventLog::new();
        let store = MockWorldStore::new();

        let mut sim = Simulation::new(rng, clock, event_log, store);

        // Create the world
        sim.process_command(Command::CreateWorld(CreateWorldCmd {
            name: self.world_name,
            seed: self.seed,
        }))
        .expect("Failed to create test world");

        // Spawn entities
        for spawn_cmd in self.entities_to_spawn {
            sim.process_command(Command::SpawnEntity(spawn_cmd))
                .expect("Failed to spawn entity");
        }

        sim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_world_scenario() {
        let sim = TestScenario::empty_world(42).build();
        assert!(sim.has_world());
    }

    #[test]
    fn world_with_entities() {
        let sim = TestScenario::empty_world(42)
            .with_resource(0, 0, 100)
            .with_creature(5, 5, 50)
            .build();

        let world = sim.world().unwrap();
        assert_eq!(world.entity_count(), 2);
    }
}
