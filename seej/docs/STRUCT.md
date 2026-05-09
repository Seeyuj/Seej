seej/
├── /docs                      # Documentation in English
│   ├── ARCHITECTURE.md
│   ├── DATA_FLOW.md
│   ├── DEPENDENCY_RULES.md
│   └── /phase1
│       ├── README.md
│       ├── BINARIES.md
│       ├── DETERMINISM.md
│       ├── EXIT_CHECKLIST.md
│       └── PERSISTENCE.md
│
├── /schemas
│   └── README.md
│
├── /server                    # Rust workspace
│   ├── README.md
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   │
│   ├── /crates
│   │   ├── /sy_types          # LEVEL 0
│   │   ├── /sy_config         # LEVEL 0
│   │   ├── /sy_protocol       # LEVEL 1 (Phase 2+, outside Phase 1 workspace)
│   │   ├── /sy_api            # LEVEL 1 (commands, events, errors, validation, persistence interfaces)
│   │   ├── /sy_core           # LEVEL 2 (world, sim, ports/)
│   │   ├── /sy_infra          # LEVEL 3 (clock, hash, rng, runtime, snapshot, store, observability)
│   │   ├── /sy_loader         # LEVEL 4 (Phase 1 placeholder; server_d wires runtime)
│   │   ├── /sy_tools          # LEVEL 3 (Phase 2+ placeholders: inspect, replay)
│   │   └── /sy_testkit        # LEVEL 3 (mocks, scenarios)
│   │
│   ├── /mods
│   │   ├── /mod_economics     # LEVEL 2b
│   │   └── /mod_physics       # LEVEL 2b
│   │
│   ├── /bin
│   │   ├── /server_d          # Main daemon
│   │   └── /sy_cli            # Admin CLI
│   │
│   └── /examples
│       ├── minimal_headless.rs
│       └── replay_demo.rs
│
└── /clients                   # Empty (reserved for later)
