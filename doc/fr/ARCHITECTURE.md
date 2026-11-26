# Architecture

> **Traduction non normative** de [`doc/ARCHITECTURE.md`](../ARCHITECTURE.md).
> La version anglaise fait foi en cas d'écart.

```text
statut : ARCHITECTURE CIBLE (D-015) — LE WORKSPACE EST UN SQUELETTE VIDE PHASE 0 (PRÊT POUR LA PHASE 1.1)
```

Ce document est le blueprint architectural unique de Seej. Il consolide les
frontières système, le modèle d'état logique, le flux d'exécution, la
séparation entre coutures internes et frontière publique ciblée, les règles de
dépendance et l'arborescence du dépôt. Les brouillons normatifs vivent dans
[CONTRACT.md](CONTRACT.md) et [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md) ;
l'historique et les mesures fondatrices vivent dans
[DECISIONS.md](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée) et
[BENCHMARKS.md](BENCHMARKS.md) ; l'ordre des phases faisant foi vit uniquement
dans [ROADMAP.md](ROADMAP.md).

## Philosophie et frontières

Seej sépare strictement les **mécanismes** (dans le noyau) des **politiques et
modèles applicatifs** (dans les plugins et consommateurs) :

- **Noyau de monde :** possède l'identité du monde, le temps simulé (`Tick`),
  l'admission ordonnée et le statut durable des commandes, l'application par
  chemin unique de *write-sets*, la vérification des racines d'état, le registre
  d'obligations, la persistance et la réplication d'autorité d'un monde.
- **Moteur de transition pur (`sy_core`) :** logique déterministe de transition
  et d'intégrité sans aucun I/O. Zéro dépendance externe.
- **Runtime de nœud (`sy_infra`, futurs `server_d` / `sy_cli`) :** assemble le
  moteur pur avec le stockage interne, l'autorité/réplication, l'ordonnancement
  et l'observation.
- **Plugins applicatifs :** possèdent les schémas de domaines, commandes,
  validateurs, migrations et politiques optionnelles (espace, économie, IA,
  multirésolution, netcode ingame).
- **Observateurs :** consommateurs lecture seule de l'état et des événements
  confirmés ; leur activité n'altère jamais les transitions canoniques.

```text
plugins applicatifs & consommateurs          opérateurs & observateurs
espace · économie · IA · multirés · netcode  CLI · monitoring · flux
                 |                                      |
                 +--- UNE FRONTIÈRE PUBLIQUE v1 CIBLE --+
                 |        (BROUILLON — NON GELÉ)        |
                                    |
                     noyau logique de monde (par monde)
         identité · tick · commandes · write-set · racines · obligations
                                    |
               +--------------------+--------------------+
               |                    |                    |
           StateStore         AuthorityLog           Executor
       (couture interne)   (couture interne)    (couture interne)
```

## Modèle d'état logique et chemin de mutation

### « Narrow waist » : tables ordonnées partitionnées + obligations

Le noyau ne définit ni `Entity`, ni `Zone`, ni coordonnées spatiales, ni
archétypes ECS, ni index secondaires automatiques. L'état d'un monde est un
ensemble de domaines propriétaires versionnés exposant deux primitives logiques :

1. **`OrderedTable` :** espace clé-valeur ordonné lexicographiquement
   `(DomainId, TableId, PartitionKey, RecordKey) -> OpaqueBytes`.
   - Un plugin simple à blob opaque utilise 1 table, 1 partition, 1 clé.
   - Un domaine transactionnel (comptes, inventaires, gouvernance) utilise des
     clés fines.
   - Une simulation dense regroupe ses enregistrements dans des partitions
     contiguës (`PartitionKey`) pour la localité cache et le futur parallélisme
     multi-cœur.
   - Un index secondaire canonique est une `OrderedTable` ordinaire possédée par
     le plugin et mise à jour dans le même *write-set* atomique ; un index
     d'observation reste hors de l'état canonique.
   - Le schéma réserve un champ extensible `TableKind` permettant d'ajouter des
     tables colonnaires (`ColumnarTable`) plus tard sans casser la v1.
2. **`ObligationRegistry` :** enveloppes de cycle de vie visibles du noyau
   `(obligation_id, owner_domain, counterparty_domain, status, due_tick, opaque_payload)`
   empêchant de détacher silencieusement un plugin ayant des obligations actives.

### Chemin de mutation unique (fin du `world.clone()` et de la dérive de replay)

L'exécution en direct et la reprise après crash sont conçues pour partager
**un seul** chemin de mutation canonique :

```text
StateView immuable (à base_cursor)
  -> évaluation batch plugin -> CandidateOverlay / WriteSet + Events + Decision
  -> validation inter-domaines sur le candidat complet
  -> confirmation durable (group commit par tick)
  -> apply(WriteSet)  [même chemin canonique en direct et en reprise]
```

- **Zéro `world.clone()` :** un échec, une base périmée ou un crash plugin ne
  rejette que le petit overlay / undo en $O(k)$.
- **Zéro duplication de logique :** `apply(WriteSet)` est l'unique mutateur
  canonique d'état pour l'exécution directe comme pour la reprise.

## Exécution, déterminisme et parallélisme

### Entropie dérivée (`EntropyScope`)

Un RNG global mutable casse l'isolation entre plugins et bloque le
parallélisme. Le cœur cible dérive l'entropie déterministe à partir de
coordonnées canoniques :

```text
EntropyScope(world_seed, entropy_epoch, tick, admission_seq | phase, domain_id)
  .fork(stable_key) -> tirages 100 % entiers (aucun flottant)
```

L'algorithme PRF/PRNG à compteur est déclaré dans le profil d'exécution (jamais
codé en dur à vie dans le contrat racine). Les compteurs canoniques sont
vérifiés (échec fermé en cas de débordement) ; les IDs sont dérivés de la cause
et les événements sont collectés localement puis numérotés dans l'ordre canonique.

### Sémantique de lecture explicite ($T-1$ vs séquentiel)

- **Commandes et transactions inter-domaines :** observent l'état séquentiel
  exact produit par les commandes précédentes dans l'ordre canonique
  `(target_tick, admission_seq)`.
- **Passes de tick autonomes** lisant un instantané immuable $T-1$ puis
  réduisant leurs intentions en ordre canonique (style BSP) : elles **doivent
  déclarer explicitement cette sémantique dès la v1**, même sur l'exécuteur
  séquentiel. Sinon, passer de 1 cœur à $N$ cœurs plus tard changerait les
  sorties observables.

### Exécuteur séquentiel batché en v1 + Deterministic Simulation Testing (DST)

La cible v1 livre un **exécuteur séquentiel batché** (1 group commit par tick ;
non encore implémenté dans le squelette Phase 0). Cela offre deux avantages :

1. Les benchmarks de laboratoire jetables montrent qu'un chemin séquentiel
   mono-thread peut tenir 10 000 commandes/s sur 1 million de clés en
   **8,62–21,74 ms p99** (médiane **6,64–9,95 ms**, voir
   [BENCHMARKS.md](BENCHMARKS.md)).
2. Il débloque le **Deterministic Simulation Testing (DST) mono-thread** —
   simuler un cluster complet de 3 nœuds, des partitions réseau, du
   réordonnancement de paquets et des pannes disque injectées de façon
   reproductible depuis une seule graine.

Le parallélisme multi-cœur (`Block-STM` pour les transactions dynamiques, `BSP`
parallèle par partition pour les ticks denses) se branche derrière la couture
interne `Executor` en Phase 4.1 (Gate 1-D) avec un oracle absolu :
`root(1 cœur) == root(N cœurs)`.

## Persistance, durabilité et réplication

### Autorité CP par monde et fédération inter-mondes

- **Dans un monde :** un seul groupe d'autorité (1 nœud local, ou 3 nœuds
  *shared-nothing* avec quorum 2 via un log de consensus de type Raft) produit
  une histoire confirmée unique. Un ajout WAL local n'est jamais une
  confirmation distribuée ; une perte de quorum suspend les écritures tout en
  autorisant une inspection lecture seule explicitement marquée *stale*.
- **Entre mondes :** la décentralisation, l'autonomie locale et le pair-à-pair
  opèrent **entre mondes indépendants** via des enveloppes asynchrones et des
  obligations durables — jamais en couplant en *lockstep* $N$ groupes
  d'autorité sur un même tick (dont la disponibilité conjointe s'effondre en
  $p^N$).
- **Les partitions d'un monde** partagent son groupe d'autorité et son horloge ;
  elles servent la localité mémoire, le cache CPU, les checkpoints incrémentaux
  et le passage à l'échelle multi-cœur intra-nœud.

### Ticks group-committés, racines et modes de preuve

Un tick confirmé scelle un lot compact de résolutions ancrées par
`pre_state_root`, `post_state_root` et `events_root` :

- **Les racines sont gelées dans le contrat cible v1 ; le mode de preuve et
  l'algorithme de hash vivent dans le profil d'exécution.**
  - **Mode de preuve par défaut v1 (`delta`) :** chaque `Resolution` porte le
    `WriteSet` canonique explicite, permettant une reprise mécanique sans
    ré-exécuter les plugins natifs (préserve la sûreté D-014 tant que les
    plugins ne sont pas isolés).
  - **Mode de preuve compact futur (`root` / SMR vérifié) :** profil opt-in pour
    les charges autonomes denses (ex. $10^5$ écritures/tick, où les benchmarks
    montrent que `delta` produit 14,4 Mo/tick et échoue au budget de 100 ms sur
    la latence de queue disque) une fois le déterminisme garanti par isolation
    ou profils vérifiés.
- **Hash d'état incrémental par enregistrement :** les benchmarks ont montré que
  le rehash complet par partition détruit les charges de commandes dispersées
  (115–169 ms médiane contre 6–10 ms). L'algorithme exact reste un paramètre de
  profil versionné.
- **Checkpoints :** `MigrationActivate` commite un checkpoint frontière
  obligatoire afin qu'une reprise ordinaire ne traverse jamais deux profils.
  Sortir l'écriture du checkpoint hors du chemin critique du tick (CoW
  asynchrone) est nécessaire à grande échelle (**237–435 ms** de gel mesurés
  pour 144 Mo synchrones, avec une pointe à **1 485 ms**).

### Trois curseurs distincts

1. `confirmed_cursor` — position la plus haute confirmée par le quorum.
2. `checkpoint_cursor` — position confirmée liée atomiquement à l'état, au
   profil d'exécution et à la racine sémantique d'un checkpoint.
3. `applied_cursor` — position confirmée la plus haute matérialisée dans une
   instance locale d'état (ne peut servir de lecture *current* tant qu'elle est
   en retard sur la barrière de lecture du quorum).

## Coutures internes vs frontière publique ciblée

Selon la règle Linux (*ABI syscall stable, aucune API interne stable pour les
pilotes*) et la règle des 3 rôles de Cordis (*une couture exige définition,
fournisseur et consommateur*) :

- **Frontière publique visée pour le gel à `K-01` (actuellement BROUILLON — NON GELÉ) :**
  - Le **Contrat Plugin, Commandes et Observation** ([CONTRACT.md](CONTRACT.md)
    et [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md)).
- **Coutures internes cibles (non gelées publiquement en v1) :**
  1. `StateStore` — coupes MVCC (`pin_cut`), `apply(WriteSet)` atomique,
     streaming de chunks, calcul de racine incrémental (cible : vérifié avec
     doubles en mémoire et injection de pannes `P0-14`).
  2. `AuthorityLog` — WAL group-commit, réplication 1 nœud / Raft 3 nœuds,
     termes d'autorité, membership conjoint, fencing et barrières de lecture
     (cible : vérifié via doubles réseau/stockage DST `K-05` / `K-07`).
  3. `Executor` — exécution séquentielle batchée en v1 ; `Block-STM` / `BSP`
     parallèle et sandboxing Wasmtime plus tard sans casser les plugins.

## Règles de dépendance et pureté

### Direction cible des dépendances

```text
primitives stables (sy_types — zéro dépendance)
        ^
brouillon de contrats publics (sy_api — dépend strictement de sy_types)
        ^
moteur déterministe pur (sy_core — dépend strictement de sy_types, sy_api)
        ^
adaptateurs d'infrastructure (sy_infra)
        ^
futurs binaires de nœud (server_d, sy_cli)
```

Les plugins applicatifs dépendent **uniquement** des contrats publics — jamais
de `sy_core`, `sy_infra`, des internes d'un autre plugin ou du runtime.

### Matrice workspace contrôlée (`seej/server/`)

| Crate | Dépendances autorisées (normal, dev et build) |
| --- | --- |
| `sy_types` | aucune (contrôlé par `scripts/check_dag.py` et CI) |
| `sy_api` | `sy_types` (contrôlé par `scripts/check_dag.py` et CI) |
| `sy_core` | `sy_types`, `sy_api` (zéro crate externe, contrôlé par `scripts/check_dag.py` et CI) |
| `sy_infra` | `sy_types`, `sy_api`, `sy_core` (+ crates externes ajoutés uniquement à leur phase) |

### Règles de pureté (`sy_types`, `sy_api`, `sy_core`)

Les trois crates purs ne doivent jamais accéder au système de fichiers, au
réseau, à l'horloge murale/processus, aux variables d'environnement, au hasard
OS, aux threads OS, aux collections hachées aléatoires (`HashMap`/`HashSet`) ni
aux types flottants non contraints (`f32`/`f64`). Contrôlé par
`seej/server/scripts/check_sy_core_purity.py` (18 auto-tests),
`#![deny(clippy::float_arithmetic)]`, `#![forbid(unsafe_code)]` et
`seej/server/scripts/check_dag.py`. Chaque crate impose `#![deny(missing_docs)]`.

### Raccourcis interdits

- Aucun layout de stockage spécifique dans les types purs.
- Aucune mutation directe par un plugin d'un domaine qu'il ne possède pas.
- Aucune dépendance aux observateurs dans les transitions canoniques.
- Aucun contrat public défini accidentellement par `serde` sur des enums Rust internes.
- Aucune migration implicite ni exécution mixte de profils dans un monde.

## Arborescence du dépôt

```text
.
├── README.md                  # vue d'ensemble & sélecteur de langue (EN par défaut)
├── AGENTS.md                  # unique guide racine des agents (carte, checks, règles)
├── CONTRIBUTING.md            # contribution et gouvernance (EN)
├── rust-toolchain.toml        # Rust 1.95.0 épinglé à la racine
├── .omp/RULES.md              # interdits absolus permanents
├── .github/workflows/rust.yml # CI PR open-source (docs, pureté, DAG, rust, supply-chain, build)
├── doc/
│   ├── ARCHITECTURE.md        # blueprint unique (EN)
│   ├── DECISIONS.md           # journal des décisions D-001 à D-015 (EN)
│   ├── ROADMAP.md             # plan de codage ordonné unique (Phase 0–4) & checklist (EN)
│   ├── SECURITY.md            # politique de sécurité (EN)
│   ├── CODE_OF_CONDUCT.md     # code de conduite
│   └── fr/                    # miroir français non normatif de la documentation
└── seej/
    ├── docs/
    │   ├── CONTRACT.md        # brouillon v1 contrat noyau & autorité (EN)
    │   ├── PLUGIN_CONTRACT.md # brouillon v1 frontière publique plugin ciblée (EN)
    │   └── BENCHMARKS.md      # mesures empiriques fondant D-015 (EN)
    └── server/                # workspace Rust vierge (squelette vide Phase 0)
        ├── Cargo.toml
        ├── deny.toml          # politique advisories, licences, bans, sources
        ├── rust-toolchain.toml
        ├── scripts/           # check.py, check_sy_core_purity.py, check_dag.py, check_docs.py
        └── crates/
            ├── sy_types/
            ├── sy_api/
            ├── sy_core/
            └── sy_infra/
```
