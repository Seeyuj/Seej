# Repository Structure

This tree describes the current checkout. Comments marked **target** indicate a
planned responsibility rather than active code.

```text
seej/
├── docs/
│   ├── ARCHITECTURE.md          # technical architecture and current mapping
│   ├── CONTRACT.md              # root draft kernel contract
│   ├── NODE_NETWORK.md          # target inter-node authority contract
│   ├── PLUGIN_CONTRACT.md       # target provider/plugin contract
│   ├── DATA_FLOW.md             # current and target transition flows
│   ├── DEPENDENCY_RULES.md      # enforced and target dependency direction
│   ├── STRUCT.md
│   ├── simulation/
│   │   ├── WORLD_SPEC.md
│   │   └── CAUSAL_RESOLUTION.md # optional community-extension contract
│   └── phase1/
│       ├── README.md
│       ├── BINARIES.md
│       ├── DETERMINISM.md
│       ├── EXIT_CHECKLIST.md
│       ├── OBSERVATION_SLICE.md
│       └── PERSISTENCE.md
├── schemas/
│   └── README.md                # public-schema policy; no stable schemas yet
├── server/
│   ├── README.md
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   ├── crates/
│   │   ├── sy_types             # NIV 0, active
│   │   ├── sy_config            # NIV 0, active
│   │   ├── sy_api               # NIV 1, active
│   │   ├── sy_core              # NIV 2, active pure transition engine
│   │   ├── sy_infra             # NIV 3, active local infrastructure
│   │   ├── sy_tools             # NIV 3, active placeholder
│   │   ├── sy_testkit           # NIV 3, active test support
│   │   ├── sy_loader            # NIV 4, inert placeholder
│   │   └── sy_protocol          # outside active workspace; target networking
│   ├── mods/
│   │   ├── mod_economics        # outside active workspace; placeholder
│   │   └── mod_physics          # outside active workspace; placeholder
│   ├── bin/
│   │   ├── server_d             # active local daemon
│   │   └── sy_cli               # inspector; init/export writes tracked by P1-11
│   └── examples/                # unwired placeholders
└── clients/                     # empty; optional ecosystem reservation
```

Future implementation may rename or regroup crates after an explicit
architecture change. This file must follow the actual tree and may not be used
to claim that a placeholder implements its target contract.
