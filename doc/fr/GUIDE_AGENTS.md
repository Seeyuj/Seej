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

## Ordre d'arbitrage & invariants fondamentaux

Arbitrer les compromis strictement dans cet ordre : **durabilité architecturale > exactitude > déterminisme > persistance et reprise > maintenabilité > testabilité > observabilité > performance > vitesse d'implémentation**.

- **Autorité CP par monde :** un groupe autorisé détient l'unique histoire confirmée d'un monde (1 nœud ou 3 nœuds *shared-nothing*, quorum 2). La découverte n'accorde jamais l'autorité ; une partition minoritaire ne peut pas écrire. Des `authority_term` durables clôturent les coordinateurs ; des `config_version` confirmées gouvernent le membership.
- **Fonctionnement headless :** les mondes évoluent sans UI, rendu, client, agent ni netcode ingame ; les déploiements 1 nœud et 3 nœuds obéissent au même contrat public.
- **Déterminisme :** mêmes genèse/checkpoint, entrées admises ordonnées, calendrier de ticks et profil d'exécution produisent des transitions identiques. Les crates purs utilisent un `EntropyScope` entier dérivé de coordonnées et ne dépendent jamais de l'horloge murale, du timing réseau, de l'ordre FS, des threads OS, du hasard ambiant ni de décisions flottantes.
- **Durabilité et persistance *fail-closed* :** ne jamais rapporter un succès canonique ni publier d'effets avant la confirmation durable requise (un ajout WAL local n'est pas un commit distribué). Échouer fermé sur erreur critique de persistance ; suspendre les mutations sur perte de quorum tout en permettant une inspection lecture seule explicitement marquée périmée (*stale*).
- **Compatibilité :** tout changement affectant le rejeu exige une version explicite de contrat ou de profil ; les plugins possèdent leurs migrations ; les plugins incompatibles bloquent l'activation ; `MigrationActivate` commite un checkpoint frontière obligatoire.

## Carte

| Chemin | Rôle | API publique | Dépendances autorisées (tous types) | Tests |
| --- | --- | --- | --- | --- |
| `seej/server/crates/sy_types/` | Primitives stables, compteurs vérifiés, DTOs de profils | aucune (squelette Phase 0) | aucune | aucun |
| `seej/server/crates/sy_api/` | Brouillon de frontière publique v1 plugin/commandes/observation | aucune (squelette Phase 0) | `sy_types` | aucun |
| `seej/server/crates/sy_core/` | Moteur de transition déterministe pur | aucune (squelette Phase 0) | `sy_types`, `sy_api` | aucun |
| `seej/server/crates/sy_infra/` | Adaptateurs persistance, autorité, exécution, DST, observation | aucune (squelette Phase 0) | `sy_types`, `sy_api`, `sy_core` | aucun |
| `seej/server/scripts/` | Validation unifiée (`check.py`), pureté, DAG, liens/parité doc | `check.py`, `check_sy_core_purity.py`, `check_dag.py`, `check_docs.py` | stdlib Python uniquement | Suite `--self-test` intégrée |

Hors source, ne pas explorer : `seej/server/target/`, `graft/`, `assets/`.

## Lecture progressive

1. [`AGENTS.md`](../../AGENTS.md) à la racine et [CONTRIBUTING.md](CONTRIBUTING.md) (discipline de contribution et critères de refus).
2. [ROADMAP.md](ROADMAP.md) (ou [`doc/ROADMAP.md`](../ROADMAP.md)) pour l'ordre unique des phases, les questions ouvertes pré-Phase 1.2 et les écarts ouverts.
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
| Crates purs (`sy_types`, `sy_api`, `sy_core`, tous `.rs` dont `build.rs`/`tests/`) sans horloge murale, env, fs/io, réseau, hasard OS, threads, processus, `HashMap`/`HashSet` ni `f32`/`f64` | `seej/server/scripts/check_sy_core_purity.py` + `#![deny(clippy::float_arithmetic)]` | lint |
| Zéro code `unsafe` dans les crates purs | `#![forbid(unsafe_code)]` dans chaque crate pur | lint |
| Tout item public documenté avec zéro warning Rustdoc | `#![deny(missing_docs)]` + `cargo doc` (`RUSTDOCFLAGS=-D warnings`) | lint |
| Politique dépendances (licences MIT/Apache/BSD, advisories, bans, sources) | `seej/server/deny.toml` (`cargo deny check advisories licenses bans sources`) | lint |
| Liens Markdown valides, ancres Unicode, unique `AGENTS.md` racine, IDs de décisions uniques, synchro toolchain, parité EN/FR | `seej/server/scripts/check_docs.py` (`check.py` + CI) | test |
| Compteurs canoniques vérifiés *fail-closed* (`P0-04`), jamais de wrap ni de saturation | aucun | prose |
| Zéro ontologie applicative dans `sy_core` ; les plugins ne dépendent jamais de `sy_core` (`P2-06`) | aucun | prose |
| Chemin de mutation unique `apply(WriteSet)` ; aucun succès avant confirmation durable par quorum | aucun | prose |
| Ne geler en v1 que le contrat public plugin/commandes/observation ; garder les coutures internes internes jusqu'à un second fournisseur et consommateur réels | aucun | prose |
| Budget de contrôle réservé pour `CancelCommand` et contrôle de migration ; ne jamais présenter une cible comme implémentée | aucun | prose |

Pour la méthode de travail complète et le format de compte rendu, consulter la référence faisant foi [`AGENTS.md`](../../AGENTS.md).
