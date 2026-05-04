//! # server_d
//!
//! Main server daemon binary - headless simulation server.
//!
//! ## Phase 1 Features
//! - Create new worlds or load existing ones
//! - Run simulation ticks continuously
//! - Auto-save at configurable intervals
//! - Graceful shutdown with save
//! - Recovery from crash (WAL replay)

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use sy_api::commands::{Command, CreateWorldCmd, EntityProperties, SpawnEntityCmd};
use sy_core::ports::IWorldStore;
use sy_core::Simulation;
use sy_infra::{FileEventLog, FilesystemStore, Pcg32Rng, UnlimitedClock};
use sy_types::{EntityKind, Position, RngSeed, WorldPos, ZoneId};

/// See-Yuj headless simulation server
#[derive(Parser)]
#[command(name = "server_d")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Data directory for world storage
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new world
    Create {
        /// World name
        #[arg(short, long)]
        name: String,

        /// RNG seed (REQUIRED for determinism)
        #[arg(short, long)]
        seed: u64,

        /// Number of initial resources to spawn
        #[arg(long, default_value = "10")]
        resources: u32,

        /// Number of initial creatures to spawn
        #[arg(long, default_value = "5")]
        creatures: u32,
    },

    /// Run simulation on an existing world
    Run {
        /// World ID to load
        #[arg(short, long)]
        world: String,

        /// Number of ticks to run (0 = infinite)
        #[arg(short, long, default_value = "0")]
        ticks: u64,

        /// Auto-save interval in ticks (0 = no auto-save)
        #[arg(long, default_value = "100")]
        save_interval: u64,
    },

    /// List available worlds
    List,
}

fn main() {
    let cli = Cli::parse();

    // Initialize logging
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    info!("See-Yuj Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Data directory: {:?}", cli.data_dir);

    // Setup shutdown signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc_handler(move || {
        warn!("Shutdown signal received");
        r.store(false, Ordering::SeqCst);
    });

    // Execute command
    let result = match cli.command {
        Commands::Create {
            name,
            seed,
            resources,
            creatures,
        } => cmd_create(&cli.data_dir, &name, seed, resources, creatures),
        Commands::Run {
            world,
            ticks,
            save_interval,
        } => cmd_run(&cli.data_dir, &world, ticks, save_interval, running),
        Commands::List => cmd_list(&cli.data_dir),
    };

    if let Err(e) = result {
        error!("Error: {}", e);
        std::process::exit(1);
    }

    info!("Server shutdown complete");
}

/// Register Ctrl+C handler
fn ctrlc_handler<F: FnOnce() + Send + 'static>(handler: F) {
    let handler = std::sync::Mutex::new(Some(handler));
    ctrlc::set_handler(move || {
        if let Some(h) = handler.lock().unwrap().take() {
            h();
        }
    })
    .expect("Failed to set Ctrl+C handler");
}

/// Create a new world
/// Create a new world with an explicit seed (required for determinism).
fn cmd_create(
    data_dir: &PathBuf,
    name: &str,
    seed: u64,
    resources: u32,
    creatures: u32,
) -> Result<(), String> {
    info!("Creating world '{}' with seed {}", name, seed);

    let mut sim = create_simulation(data_dir, &format!("world_{}", seed))?;

    // Create the world
    sim.process_command(Command::CreateWorld(CreateWorldCmd {
        name: name.to_string(),
        seed: RngSeed::new(seed),
    }))
    .map_err(|e| format!("Failed to create world: {}", e))?;

    let world_id = sim.world().unwrap().id().to_string();
    info!("World created with ID: {}", world_id);

    // Spawn initial entities
    for i in 0..resources {
        let x = (i as i32 % 10) * 10;
        let y = (i as i32 / 10) * 10;

        sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new(x, y, 0)),
            kind: EntityKind::Resource,
            properties: EntityProperties {
                name: Some(format!("Resource_{}", i)),
                amount: Some(100),
                health: None,
            },
        }))
        .map_err(|e| format!("Failed to spawn resource: {}", e))?;
    }

    for i in 0..creatures {
        let x = (i as i32 % 10) * 10 + 5;
        let y = (i as i32 / 10) * 10 + 5;

        sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new(x, y, 0)),
            kind: EntityKind::Creature,
            properties: EntityProperties {
                name: Some(format!("Creature_{}", i)),
                amount: None,
                health: Some(100),
            },
        }))
        .map_err(|e| format!("Failed to spawn creature: {}", e))?;
    }

    // Final save
    sim.process_command(Command::SaveWorld)
        .map_err(|e| format!("Failed to save world: {}", e))?;

    info!(
        "World '{}' created with {} resources and {} creatures",
        name, resources, creatures
    );

    Ok(())
}

/// Run simulation
fn cmd_run(
    data_dir: &PathBuf,
    world_id: &str,
    max_ticks: u64,
    save_interval: u64,
    running: Arc<AtomicBool>,
) -> Result<(), String> {
    info!("Loading world '{}'", world_id);

    let mut sim = create_simulation(data_dir, world_id)?;

    // Load the world
    sim.process_command(Command::LoadWorld(sy_api::commands::LoadWorldCmd {
        world_id: world_id.to_string(),
    }))
    .map_err(|e| format!("Failed to load world: {}", e))?;

    let start_tick = sim.current_tick();
    info!("World loaded at tick {}", start_tick);

    let mut ticks_run = 0u64;
    let mut last_save_tick = start_tick.as_u64();

    // Main simulation loop
    info!("Starting simulation loop...");

    while running.load(Ordering::SeqCst) {
        // Check tick limit
        if max_ticks > 0 && ticks_run >= max_ticks {
            info!("Reached tick limit ({})", max_ticks);
            break;
        }

        // Run one tick
        let events = sim
            .process_command(Command::Tick)
            .map_err(|e| format!("Tick failed: {}", e))?;

        ticks_run += 1;
        let current_tick = sim.current_tick();

        // Log progress periodically
        if current_tick.as_u64() % 100 == 0 {
            let world = sim.world().unwrap();
            info!(
                "Tick {} | Entities: {} active | Events: {}",
                current_tick,
                world.active_entity_count(),
                events.len()
            );
        }

        // Auto-save
        if save_interval > 0 && (current_tick.as_u64() - last_save_tick) >= save_interval {
            info!("Auto-saving at tick {}...", current_tick);
            sim.process_command(Command::SaveWorld)
                .map_err(|e| format!("Auto-save failed: {}", e))?;
            last_save_tick = current_tick.as_u64();
        }
    }

    // Final save on shutdown
    info!("Saving world before shutdown...");
    sim.process_command(Command::Shutdown)
        .map_err(|e| format!("Shutdown save failed: {}", e))?;

    let final_tick = sim.current_tick();
    info!(
        "Simulation complete. Ran {} ticks (from {} to {})",
        ticks_run, start_tick, final_tick
    );

    Ok(())
}

/// List available worlds
fn cmd_list(data_dir: &PathBuf) -> Result<(), String> {
    let store =
        FilesystemStore::new(data_dir).map_err(|e| format!("Failed to open store: {}", e))?;

    let worlds = store
        .list_worlds()
        .map_err(|e| format!("Failed to list worlds: {}", e))?;

    if worlds.is_empty() {
        println!("No worlds found in {:?}", data_dir);
    } else {
        println!("Available worlds:");
        for world_id in worlds {
            if let Ok(meta) = store.load_meta(&world_id) {
                println!(
                    "  {} - '{}' (tick {}, seed {})",
                    world_id,
                    meta.name,
                    meta.current_tick,
                    meta.seed.as_u64()
                );
            } else {
                println!("  {} (metadata unavailable)", world_id);
            }
        }
    }

    Ok(())
}

/// Create a simulation instance with real infrastructure
fn create_simulation(
    data_dir: &PathBuf,
    world_id: &str,
) -> Result<Simulation<Pcg32Rng, UnlimitedClock, FileEventLog, FilesystemStore>, String> {
    let store =
        FilesystemStore::new(data_dir).map_err(|e| format!("Failed to create store: {}", e))?;

    let events_dir = store.events_dir(world_id);
    let event_log =
        FileEventLog::new(&events_dir).map_err(|e| format!("Failed to create event log: {}", e))?;

    let rng = Pcg32Rng::new(RngSeed::new(0)); // Will be set from world seed
    let clock = UnlimitedClock::new();

    Ok(Simulation::new(rng, clock, event_log, store))
}
