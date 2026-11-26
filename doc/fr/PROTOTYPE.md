# Référence du prototype local actuel

> *Miroir français de [`seej/docs/PROTOTYPE.md`](../../seej/docs/PROTOTYPE.md).
> La version anglaise fait foi en cas d'écart.*

```text
status: PROTOTYPE LOCAL IMPLÉMENTÉ — BASE REMPLAÇABLE (PAS LE CONTRAT v1)
```

Ce document décrit ce qui tourne réellement dans `seej/server/` aujourd'hui :
binaires CLI, formats sur disque, vérifications de déterminisme/pureté, limites
connues et mesures empiriques issues du harnais de charges jetable.

Selon [D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée), les
types fixes `Entity`/`Zone`, les règles de dégradation, les snapshots JSON et
le runtime mono-écrivain de ce prototype sont des **fixtures de référence**,
pas l'API cible du noyau v1. Reconstruire proprement le cœur générique v1 à
partir de zéro est explicitement préféré à l'étirement de ces types.

---

## Périmètre et politique de remplacement

Implémenté aujourd'hui dans `seej/server/` :

- démon local headless mono-processus `server_d` et CLI d'inspection `sy_cli` ;
- création de monde avec `--seed` explicite et RNG initial mélangé par
  SplitMix64 (format v4), avec continuation rétrocompatible des mondes en
  format v3 ;
- exécution déterministe de ticks sur des collections `BTreeMap` ;
- WAL v1 *fail-closed* (enregistrements batch protégés par CRC32) + snapshots
  JSON atomiques ;
- rejeu strict d'événements et rollback de lot `TickN` ;
- porte de pureté statique sur `sy_core` (`scripts/check_sy_core_purity.py`),
  vérification des dépendances workspace, `proptest` de parité de reprise, test
  d'arrêt forcé, *burn-in* 100k et cibles fuzz WAL isolées.

Non implémenté dans ce prototype : domaines plugins génériques, `OrderedTable` /
`ObligationRegistry`, chemin unique `apply(WriteSet)`, consensus distribué,
observation réseau, enveloppes durables de commandes et migrations de schémas.

---

## Binaires et commandes

Toutes les commandes s'exécutent depuis `seej/server/` avec la chaîne Rust
épinglée (`rust-toolchain.toml`) :

```bash
cargo build --workspace
cargo test --workspace --all-targets
```

### `server_d`

```bash
# Créer un monde de démonstration seedé (graine obligatoire ; crée un format v4)
cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- create --name "MyWorld" --seed 42 --resources 10 --creatures 5

# Lister les mondes locaux
cargo run --bin server_d -- list

# Exécuter la simulation (--ticks 0 tourne jusqu'à Ctrl+C avec sauvegarde propre ; --save-interval en ticks)
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100

# Changer le répertoire de stockage (par défaut : ./data)
cargo run --bin server_d -- --data-dir ./my_data list
```

### `sy_cli`

```bash
cargo run --bin sy_cli -- status world_42
cargo run --bin sy_cli -- events world_42 --count 50
cargo run --bin sy_cli -- events world_42 --from-tick 1000 --count 50
cargo run --bin sy_cli -- dump world_42 --pretty
cargo run --bin sy_cli -- entities world_42 --kind creature
cargo run --bin sy_cli -- entity world_42 1
cargo run --bin sy_cli -- zones world_42
```

---

## Stockage et format WAL

Avec `--data-dir <BASE>` et `<world_id>` (actuellement dérivé de la graine
`world_<seed>`) :

```text
<BASE>/worlds/<world_id>/
  meta.json      # miroir de WorldMeta
  snapshot.json  # WorldSnapshotDto sérialisé + curseur de reprise
  events         # fichier WAL binaire v1 (payloads batch JSON + CRC32)
```

### Formats de snapshot (v3 / v4)

- **Format v4 (défaut actuel pour les nouveaux mondes) :** initialise
  `rng_state` avec le finaliseur de SplitMix64 sur la graine (corrigeant le
  défaut v3 où toute graine brute `< 2^27` produisait `0` au premier tirage
  PCG32 `next_u32()`).
- **Format v3 (lecture/continuation héritée) :** chargé et poursuivi sans
  migration ; la sauvegarde conserve la version de format existante du monde
  (`3` ou `4`). Les autres versions sont refusées.
- Les écritures de snapshot et `meta.json` passent par fichier temporaire +
  `sync_all` + renommage atomique (`MoveFileExW` avec
  `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` sous Windows ; `rename`
  + sync du répertoire parent sous Unix).

### Format binaire d'enregistrement WAL v1

```text
MAGIC    u32 LE   0x57414C31 ("WAL1")
VERSION  u16 LE   1
LENGTH   u32 LE   longueur du payload (1..=16 Mio)
EVENT_ID u64 LE   dernier ID d'événement du lot (non nul)
TICK     u64 LE   tick du dernier événement du lot
PAYLOAD  bytes    JSON {"type":"Batch","data":{"events":[...]}}
CRC32    u32 LE   CRC32 de MAGIC jusqu'à PAYLOAD inclus
```

### Comportement *fail-closed* à la reprise WAL

À une frontière d'enregistrement validée lors de l'ouverture normale
(`FileEventLog::new`) :

- **EOF exact** (ou fichier absent) : réussit sans modification.
- **Fragment terminal de 1 à 25 octets** compatible avec un préfixe d'en-tête
  v1 (octets `magic`/`version` conformes, longueur prolongeable dans
  `1..=16 Mio`, `event_id` non nul s'il est complet) : tronque uniquement ce
  fragment et appelle `sync_all`.
- **Tout le reste échoue fermé avant la moindre écriture :** en-tête complet de
  26 octets avec payload/CRC manquant, version WAL non supportée, mauvais
  `magic`, erreur CRC, lot illisible ou incohérent, ou erreur OS de lecture /
  seek / métadonnées.
- **Inspection lecture seule (`FileEventLog::open_read_only`) :** ne tronque
  jamais le fichier et propage toute erreur de scan enregistrée ou observée à la
  lecture.

La reprise charge `snapshot.json`, valide `meta.json` (autorisant un miroir
absent ou strictement plus ancien pour le même monde/graine/format/tick de
création), valide la contiguïté des IDs WAL depuis `1` couvrant
`snapshot.last_event_id`, puis rejoue les événements postérieurs au curseur du
snapshot via `sy_core::replay::apply_event`.

---

## Déterminisme et porte de pureté

Exécuter depuis `seej/server/` :

```bash
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
cargo test -p sy_core
```

Le script de pureté masque les commentaires, chaînes, chaînes brutes et
littéraux caractères Rust (y compris `'"'` et `b'"'`) avant d'inspecter
`crates/sy_core/src` pour interdire horloge murale, variables d'environnement,
système de fichiers / I/O, réseau, hasard OS et collections hachées aléatoires.
La CI vérifie en outre que `sy_core` dépend strictement de `sy_types` et
`sy_api` avec zéro crate externe.

---

## Limites connues du prototype

Limitations délibérées ou suivies du prototype (pas des garanties de
production) :

- **Clonage intégral du monde :** `PersistentSimulation::process_command`,
  `Simulation::process_command` et `replay::apply_event` clonent tout le
  `World` pour le rollback ; `save_world` clone et encode tout le monde en
  JSON.
- **Double chemin de mutation :** `sim.rs` mute directement `World` tandis que
  `replay.rs` réimplémente séparément chaque application d'événement.
- **Décisions flottantes & compteurs saturants :** la dégradation utilise
  `rng.chance(f32)` (`P0-03`), `Tick::next` / `EventId::next` saturent
  (`P0-04`) et `compute_canonical_hash` hache des curseurs de stockage avec
  l'état du monde via xxHash64 non cryptographique (`P1-04`).
- **Absence de verrou d'écrivain exclusif (`P0-10`) & noms `.tmp` statiques :**
  lancer deux processus `server_d` sur le même répertoire de monde est dangereux.
- **Aucune compaction WAL :** le WAL croît indéfiniment (`truncate_after` est
  déprécié et ne doit pas être utilisé automatiquement, `P1-18`/`K-10`).
- **Réserves d'inspection (`P1-11`) :** `sy_cli` ouvre le WAL en lecture seule,
  mais `FilesystemStore::new` peut créer `<data-dir>/worlds`, `dump --output`
  ne bloque pas les chemins faisant alias sur le stockage canonique et les
  lectures concurrentes pendant que `server_d` écrit ne sont pas supportées.
- **Aucune suite d'injection de pannes I/O (`P0-14`) :** `FileEventLog`
  n'empoisonne pas son `BufWriter` après une erreur d'écriture partielle
  (`server_d` quitte à la première erreur, mais un appelant bibliothèque ne
  doit pas réutiliser une instance de log en échec).
- **Couverture OS des tests CI :** les tests unitaires/intégration tournent
  actuellement sur `ubuntu-latest` ; les jobs Windows et macOS n'exécutent que
  `cargo build --release`.

---

## Mesures de benchmarks (harnais de charges)

Pour ancrer [D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée)
dans des mesures avant d'écrire le nouveau cœur, un harnais autonome jetable
(`seej_lab_workloads` sur les branches `lab/wal-fail-closed-load-b` `1c98282`
et `lab/state-layout-b` `5856396`, hors workspace/CI) a mesuré trois charges
synthétiques sur **1 000 000 clés × 128 o** (1 000 partitions × 1 000
enregistrements), **budget de tick 10 Hz / 100 ms**, en mono-thread sur un AMD
Ryzen 9 5900X + NVMe Crucial P2 1 To (Windows 11, `rustc 1.95.0 --release`),
avec 24/24 reprises (checkpoint + 200 ticks) vérifiées par racine :

- **Charge A (commandes) :** 1 000 commandes/tick (10 000 cmd/s), 0 écriture
  autonome.
- **Charge B (autonome dense) :** 100 000 écritures autonomes/tick
  (1 M écritures/s), 0 commande.
- **Charge C (mixte + points chauds) :** 1 000 commandes + 1 000 écritures
  autonomes/tick, 80 % frappant 32 clés chaudes.

### Synthèse des résultats mesurés

| Charge & variante | `delta` p99 (ms) | `root` p99 (ms) | `delta` octets/tick | `root` octets/tick | Meilleur rejeu (ms/tick) |
| --- | ---: | ---: | ---: | ---: | ---: |
| **A** — `btree/acc` | 10,0–10,8 | 8,8–9,2 | 417 Ko (4,2 Mo/s) | 129 Ko (1,3 Mo/s) | 2,78 (`delta`) |
| **A** — `partitioned/acc-cached` | **8,6** | **7,6** | 417 Ko | 129 Ko | **2,06** (`delta`) |
| **A** — `partitioned/rehash` | 162,6 | 166,8 | 417 Ko | 129 Ko | — (cassé par le rehash par cmd) |
| **C** — `btree/acc` | 11,8–12,4 | 10,7–11,7 | 561 Ko (5,6 Mo/s) | 129 Ko | 3,35 (`delta`) |
| **C** — `partitioned/acc-cached` | 21,7* (p50 **8,4**) | 29,4* (p50 **7,6**) | 561 Ko | 129 Ko | **~2,5–3,7** |
| **B** — `btree/acc` | 161–445 | 140–147 | **14,4 Mo (144 Mo/s)** | **89 o** | 113–129 (ratio > 1,1) |
| **B** — `partitioned/acc` | 130–348 | **95,1** | 14,4 Mo | 89 o | 70–82 |
| **B** — `partitioned/acc-cached` | 279–383 | **79,0** (p50 **66,1**) | 14,4 Mo | 89 o | **54,6** (`delta`) / **61,8** (`root`) |
| **B** — `partitioned/rehash` | 113–483 | 98,5 | 14,4 Mo | 89 o | 73,7–77,7 |

*\*Sur C (`partitioned/acc-cached`), toutes les phases CPU sont restées ≤ 7,6 ms max ; le saut du p99 provient de deux pointes de latence NVMe `sync_data` de 50–60 ms décalées au rang 288/290 par le checkpoint au tick 100.*

### Enseignements empiriques clés

1. **A et C passent avec une marge ~10× en mode `delta`** sur un seul thread,
   validant `delta` comme valeur par défaut sûre en v1 tant que les plugins
   sont en Rust natif non isolé.
2. **La charge autonome dense B échoue en `delta` sur la latence de queue
   disque** (14,4 Mo/tick, sync p99 jusqu'à 283–400 ms ; 144 Mo/s dépassent
   1 Gbps avant réplication), confirmant la nécessité de garder le mode de
   preuve configurable par profil.
3. **Le stockage contigu par partition réduit le p99 de B en `root` de 147 à
   79 ms** sur un cœur ; le rehash complet par partition casse les charges de
   commandes dispersées (A/C à 115–169 ms) lorsque des racines par résolution
   sont maintenues.
4. **Les checkpoints complets synchrones (144 Mo) bloquent chaque série pendant
   237–435 ms** (une pointe à 1 485 ms), confirmant que les checkpoints doivent
   sortir du chemin du tick en Gate 1-D.
5. **Limites de ces mesures :** mono-thread, état en RAM, accumulateur BLAKE3
   additif expérimental (pas un schéma cryptographique de production), cache OS
   chaud à la reprise, pas de réseau/quorum et aucun moteur embarqué
   (`redb`/`Fjall`) testé pour l'instant.
