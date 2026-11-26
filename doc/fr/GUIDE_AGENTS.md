# Seej — Guide des agents (traduction FR non normative)

> **Traduction non normative** de [`AGENTS.md`](../../AGENTS.md). `AGENTS.md` à
> la racine du dépôt est l'unique fichier d'instructions lu par les agents et
> fait foi en cas d'écart.

Seej est une infrastructure open-source pour des mondes bac à sable
déterministes et persistants avec un cœur minimal et une frontière publique
plugin visée pour la v1 (D-015), actuellement au stade de **squelette vierge
Phase 0** (aucun runtime applicatif). Stack : Rust `1.95.0` (épinglé dans
`rust-toolchain.toml`, zéro dépendance externe Cargo), scripts Python 3 stdlib
(`seej/server/scripts/`), `cargo-deny`. La documentation faisant foi, le code
et les commits sont en **anglais** (miroir français non normatif dans
`doc/fr/`).

- Points d'entrée : `seej/server/Cargo.toml` (workspace), [ROADMAP.md](ROADMAP.md) / [`doc/ROADMAP.md`](../ROADMAP.md) (plan de codage ordonné faisant foi). Aucun binaire produit n'existe encore.
- Le code, la configuration et les schémas exécutables font foi : signaler tout écart avec la documentation au lieu de le reproduire.

## Carte

| Chemin | Rôle | API publique | Dépendances autorisées (tous types) | Tests |
| --- | --- | --- | --- | --- |
| `seej/server/crates/sy_types/` | Primitives stables, compteurs vérifiés, DTOs de profils | aucune (squelette Phase 0) | aucune | aucun |
| `seej/server/crates/sy_api/` | Brouillon de frontière publique v1 plugin/commandes/observation | aucune (squelette Phase 0) | `sy_types` | aucun |
| `seej/server/crates/sy_core/` | Moteur de transition déterministe pur | aucune (squelette Phase 0) | `sy_types`, `sy_api` | aucun |
| `seej/server/crates/sy_infra/` | Adaptateurs persistance, autorité, exécution, DST, observation | aucune (squelette Phase 0) | `sy_types`, `sy_api`, `sy_core` | aucun |
| `seej/server/scripts/` | Validation unifiée (`check.py`), pureté, DAG, liens/parité doc | `check.py`, `check_sy_core_purity.py`, `check_dag.py`, `check_docs.py` | stdlib Python uniquement | `--self-test` intégré (18 cas) |

Hors source, ne pas explorer : `seej/server/target/`, `graft/`, `assets/`.

## Lecture progressive

1. [`AGENTS.md`](../../AGENTS.md) à la racine.
2. [ROADMAP.md](ROADMAP.md) (ou [`doc/ROADMAP.md`](../ROADMAP.md)) pour l'ordre unique des phases et les écarts ouverts.
3. [CONTRACT.md](CONTRACT.md) et [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md) dès qu'une frontière publique change ; [ARCHITECTURE.md](ARCHITECTURE.md), [DECISIONS.md](DECISIONS.md) et [BENCHMARKS.md](BENCHMARKS.md) pour les justifications.
4. Le crate propriétaire dans la carte (`src/lib.rs` et symboles ciblés), puis ses tests.

## Validation

| But | Commande (PowerShell et POSIX depuis la racine du dépôt) |
| --- | --- |
| Tout valider (local & CI) | `python3 -B seej/server/scripts/check.py` |
| Tests ciblés sur un crate | `cargo test --manifest-path seej/server/Cargo.toml --locked -p sy_core` |
| Liens, ancres et parité doc seuls | `python3 -B seej/server/scripts/check_docs.py` |

Les commandes s'exécutent à l'identique depuis la racine sous PowerShell et sous Linux/macOS (`rust-toolchain.toml` épingle `1.95.0`). `.github/workflows/rust.yml` exécute les mêmes contrôles plus `cargo audit` et les builds multi-OS sur chaque PR. `cargo-audit` n'est pas installé localement (CI uniquement). Un contrôle ignoré ou non exécuté n'est pas un succès : le signaler explicitement.

## Règles et mécanismes

| Règle | Mécanisme | Niveau |
| --- | --- | --- |
| `sy_types` = ∅, `sy_api` = `{sy_types}`, `sy_core` = `{sy_types, sy_api}` (normal, dev, build) | `seej/server/scripts/check_dag.py` (`check.py` + CI) | lint |
| Crates purs (`sy_types`, `sy_api`, `sy_core`) sans horloge murale, env, fs/io, réseau, hasard OS, threads, `HashMap`/`HashSet` ni `f32`/`f64` | `seej/server/scripts/check_sy_core_purity.py` (18 auto-tests) + `#![deny(clippy::float_arithmetic)]` | lint |
| Zéro code `unsafe` dans les crates purs | `#![forbid(unsafe_code)]` dans chaque crate pur | lint |
| Tout item public documenté avec zéro warning Rustdoc | `#![deny(missing_docs)]` + `cargo doc` (`RUSTDOCFLAGS=-D warnings`) | lint |
| Politique dépendances (licences MIT/Apache/BSD, advisories, bans, sources) | `seej/server/deny.toml` (`cargo deny check advisories licenses bans sources`) | lint |
| Liens Markdown valides, ancres Unicode, unique `AGENTS.md` racine, parité EN/FR | `seej/server/scripts/check_docs.py` (`check.py` + CI) | test |
| Compteurs canoniques vérifiés *fail-closed* (`P0-04`) | aucun | prose |
| Zéro ontologie applicative dans `sy_core` | aucun | prose |
| Chemin de mutation unique `apply(WriteSet)` ; aucun succès avant confirmation durable | aucun | prose |
| Ne geler en v1 que le contrat public ; garder les coutures internes internes | aucun | prose |
| Budget de contrôle réservé pour `CancelCommand` ; checkpoint frontière sur `MigrationActivate` | aucun | prose |
| Ne jamais présenter une cible comme implémentée | aucun | prose |

Pour la méthode de travail complète et le format de compte rendu, consulter la référence faisant foi [`AGENTS.md`](../../AGENTS.md).
