//! # sy_cli
//!
//! Admin CLI for operators - observe and inspect worlds without a UI.
//!
//! ## Commands
//! - `status`: Show world status and statistics
//! - `dump`: Dump world state to JSON
//! - `events`: List recent events
//! - `entity`: Inspect a specific entity

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use sy_core::ports::IEventLog;
use sy_core::ports::IWorldStore;
use sy_core::World;
use sy_infra::{FileEventLog, FilesystemStore};
use sy_types::EntityId;

/// See-Yuj CLI - World inspection and administration
#[derive(Parser)]
#[command(name = "sy_cli")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Data directory
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show world status and statistics
    Status {
        /// World ID
        world: String,
    },

    /// Dump world state to JSON
    Dump {
        /// World ID
        world: String,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pretty print JSON
        #[arg(short, long)]
        pretty: bool,
    },

    /// List recent events
    Events {
        /// World ID
        world: String,

        /// Number of events to show
        #[arg(short, long, default_value = "20")]
        count: usize,

        /// Start from tick
        #[arg(long)]
        from_tick: Option<u64>,
    },

    /// Inspect a specific entity
    Entity {
        /// World ID
        world: String,

        /// Entity ID
        entity_id: u64,
    },

    /// List all entities
    Entities {
        /// World ID
        world: String,

        /// Filter by kind (resource, creature, item, structure)
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// List zones
    Zones {
        /// World ID
        world: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Status { world } => cmd_status(&cli.data_dir, &world),
        Commands::Dump {
            world,
            output,
            pretty,
        } => cmd_dump(&cli.data_dir, &world, output, pretty),
        Commands::Events {
            world,
            count,
            from_tick,
        } => cmd_events(&cli.data_dir, &world, count, from_tick),
        Commands::Entity { world, entity_id } => cmd_entity(&cli.data_dir, &world, entity_id),
        Commands::Entities { world, kind } => cmd_entities(&cli.data_dir, &world, kind),
        Commands::Zones { world } => cmd_zones(&cli.data_dir, &world),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Load world from storage
fn load_world(data_dir: &PathBuf, world_id: &str) -> Result<World, String> {
    let store =
        FilesystemStore::new(data_dir).map_err(|e| format!("Failed to open store: {}", e))?;

    if !store.exists(world_id) {
        return Err(format!("World not found: {}", world_id));
    }

    let snapshot = store
        .load_snapshot(world_id)
        .map_err(|e| format!("Failed to load snapshot: {}", e))?;

    World::from_bytes(&snapshot).map_err(|e| format!("Failed to deserialize world: {}", e))
}

/// Show world status
fn cmd_status(data_dir: &PathBuf, world_id: &str) -> Result<(), String> {
    let store =
        FilesystemStore::new(data_dir).map_err(|e| format!("Failed to open store: {}", e))?;

    let meta = store
        .load_meta(world_id)
        .map_err(|e| format!("Failed to load metadata: {}", e))?;

    let world = load_world(data_dir, world_id)?;

    // Get event log info
    let events_dir = store.events_dir(world_id);
    let wal_event_count = if events_dir.exists() {
        FileEventLog::new(&events_dir)
            .map(|log| log.len())
            .unwrap_or(0)
    } else {
        0
    };

    println!("=== World Status ===");
    println!("ID:              {}", meta.world_id);
    println!("Name:            {}", meta.name);
    println!("Seed:            {}", meta.seed.as_u64());
    println!("Current Tick:    {}", meta.current_tick);
    println!("Sim Time:        {}", meta.sim_time);
    println!("Created Tick:    {}", meta.created_tick);
    println!();
    println!("=== Crash Recovery Info ===");
    println!("Snapshot Tick:   {}", meta.snapshot_tick);
    println!("Last Event ID:   {}", meta.last_event_id);
    println!("WAL Events:      {}", wal_event_count);
    println!();
    println!("=== Statistics ===");
    println!("Total Entities:  {}", world.entity_count());
    println!("Active Entities: {}", world.active_entity_count());
    println!("Zones:           {}", world.zone_count());
    println!();

    // Entity breakdown by kind
    let mut resources = 0;
    let mut creatures = 0;
    let mut items = 0;
    let mut structures = 0;

    for entity in world.entities.values() {
        match entity.kind {
            sy_types::EntityKind::Resource => resources += 1,
            sy_types::EntityKind::Creature => creatures += 1,
            sy_types::EntityKind::Item => items += 1,
            sy_types::EntityKind::Structure => structures += 1,
            _ => {} // Future entity kinds
        }
    }

    println!("=== Entity Breakdown ===");
    println!("Resources:  {}", resources);
    println!("Creatures:  {}", creatures);
    println!("Items:      {}", items);
    println!("Structures: {}", structures);

    Ok(())
}

/// Dump world state to JSON
fn cmd_dump(
    data_dir: &PathBuf,
    world_id: &str,
    output: Option<PathBuf>,
    pretty: bool,
) -> Result<(), String> {
    let world = load_world(data_dir, world_id)?;

    let json = if pretty {
        serde_json::to_string_pretty(&world)
    } else {
        serde_json::to_string(&world)
    }
    .map_err(|e| format!("Failed to serialize: {}", e))?;

    if let Some(path) = output {
        std::fs::write(&path, &json).map_err(|e| format!("Failed to write file: {}", e))?;
        println!("World dumped to {:?}", path);
    } else {
        println!("{}", json);
    }

    Ok(())
}

/// List recent events
fn cmd_events(
    data_dir: &PathBuf,
    world_id: &str,
    count: usize,
    from_tick: Option<u64>,
) -> Result<(), String> {
    let store =
        FilesystemStore::new(data_dir).map_err(|e| format!("Failed to open store: {}", e))?;

    let events_dir = store.events_dir(world_id);
    let event_log =
        FileEventLog::new(&events_dir).map_err(|e| format!("Failed to open event log: {}", e))?;

    let events = event_log
        .read_all_valid()
        .map_err(|e| format!("Failed to read events: {}", e))?;

    // Filter by tick if specified
    let filtered: Vec<_> = if let Some(tick) = from_tick {
        events
            .into_iter()
            .filter(|e| e.tick.as_u64() >= tick)
            .collect()
    } else {
        events
    };

    let total = filtered.len();
    let display_events: Vec<_> = filtered.into_iter().rev().take(count).collect();

    println!(
        "=== Events (showing {} of {}) ===",
        display_events.len(),
        total
    );

    for event in display_events.iter().rev() {
        println!("[{} | {}] {:?}", event.event_id, event.tick, event.data);
    }

    Ok(())
}

/// Inspect a specific entity
fn cmd_entity(data_dir: &PathBuf, world_id: &str, entity_id: u64) -> Result<(), String> {
    let world = load_world(data_dir, world_id)?;

    let id = EntityId::new(entity_id);
    let entity = world
        .get_entity(id)
        .ok_or_else(|| format!("Entity not found: {}", entity_id))?;

    println!("=== Entity {} ===", entity.id);
    println!("Kind:       {}", entity.kind);
    println!("State:      {:?}", entity.state);
    println!("Position:   {}", entity.position);
    println!("Created At: {}", entity.created_at);
    println!();
    println!("=== Properties ===");
    if let Some(name) = &entity.properties.name {
        println!("Name:   {}", name);
    }
    if let Some(amount) = entity.properties.amount {
        println!("Amount: {}", amount);
    }
    if let Some(health) = entity.properties.health {
        println!("Health: {}", health);
    }

    Ok(())
}

/// List all entities
fn cmd_entities(
    data_dir: &PathBuf,
    world_id: &str,
    kind_filter: Option<String>,
) -> Result<(), String> {
    let world = load_world(data_dir, world_id)?;

    let kind_filter = kind_filter.map(|s| s.to_lowercase());

    println!("=== Entities ===");
    println!(
        "{:>8} | {:>10} | {:>8} | {:>20} | {:>10}",
        "ID", "Kind", "State", "Position", "Name"
    );
    println!("{}", "-".repeat(70));

    for entity in world.entities.values() {
        let kind_str = format!("{}", entity.kind).to_lowercase();

        if let Some(ref filter) = kind_filter {
            if !kind_str.contains(filter) {
                continue;
            }
        }

        let name = entity.properties.name.as_deref().unwrap_or("-");
        println!(
            "{:>8} | {:>10} | {:>8} | {:>20} | {:>10}",
            entity.id.as_u64(),
            entity.kind,
            format!("{:?}", entity.state),
            format!("{}", entity.position),
            name
        );
    }

    Ok(())
}

/// List zones
fn cmd_zones(data_dir: &PathBuf, world_id: &str) -> Result<(), String> {
    let world = load_world(data_dir, world_id)?;

    println!("=== Zones ===");
    println!(
        "{:>8} | {:>20} | {:>8} | {:>10}",
        "ID", "Name", "Loaded", "Entities"
    );
    println!("{}", "-".repeat(55));

    for zone in world.zones.values() {
        let name = zone.name.as_deref().unwrap_or("-");
        println!(
            "{:>8} | {:>20} | {:>8} | {:>10}",
            zone.id.as_u32(),
            name,
            if zone.loaded { "Yes" } else { "No" },
            zone.entities.len()
        );
    }

    Ok(())
}
