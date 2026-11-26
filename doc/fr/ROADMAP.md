# Feuille de route et checklist de sortie

> *Miroir français de [`doc/ROADMAP.md`](../ROADMAP.md). La version anglaise
> fait foi en cas d'écart.*

Ce document est l'autorité unique pour le séquencement du projet, le statut
d'implémentation et les preuves de sortie. Les contrats de conception dans
`seej/docs/` (et leurs miroirs ici) décrivent des cibles et ne ferment jamais
un item de checklist.

## Convention de statut

- `[x]` — preuve reproductible commitée dans ce dépôt.
- `[ ]` — cible ouverte, quel que soit le niveau de détail de sa spécification.
- Les identifiants d'écarts stables (`K-*`, `P0-*`, `P1-*`, `P2-*`) sont
  préservés à travers les réorganisations ; les items retirés sont signalés
  explicitement avec leur identifiant de décision.
- **Règle d'exécution ([D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée)) :**
  construire et vérifier le Jalon 1 avant de démarrer le Jalon 2, et le Jalon 2
  avant le Jalon 3. Démarrer proprement le nouveau noyau générique (en
  remplacement du prototype spatial historique dans `seej/server/`) est
  explicitement préféré au maintien des types du prototype.

---

## Preuves implémentées du prototype

Preuves présentes dans le prototype spatial local actuel (`seej/server/`). Ces
vérifications ne prouvent que les mécaniques locales du prototype ; elles ne
ferment **pas** les items du cœur générique ni les items distribués de
Gate 1-K.

| Fait | Critère | Preuve | Reproduire depuis `seej/server/` |
| --- | --- | --- | --- |
| [x] | `P0-01` porte de pureté | [`scripts/check_sy_core_purity.py`](../../seej/server/scripts/check_sy_core_purity.py) (16 cas d'auto-test, dont littéraux `char`) + contrôle CI des dépendances | `python3 scripts/check_sy_core_purity.py --self-test` && `python3 scripts/check_sy_core_purity.py` |
| [x] | `P1-01` CI du dépôt | [`.github/workflows/rust.yml`](../../.github/workflows/rust.yml) | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --all-targets`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` |
| [x] | `P1-02` chaîne épinglée | [`rust-toolchain.toml`](../../seej/server/rust-toolchain.toml) | `rustc --version` |
| [x] | Boucle déterministe du prototype | `sy_core/src/determinism.rs`, `sim.rs` (rollback `TickN`, genèse SplitMix64 v4) | `cargo test -p sy_core` |
| [x] | RNG récupérable (v3/v4) | `sy_infra/src/rng/mod.rs`, `recovery_determinism.rs` | `cargo test -p sy_infra rng::` |
| [x] | Ouverture WAL v1 *fail-closed* | `sy_infra/src/store/wal.rs` (réparation uniquement d'un préfixe d'en-tête court 1–25 o ; refus ≥ 26 o / version / CRC / erreur I/O) + tests CLI `server_d` | `cargo test -p sy_infra store::wal`; `cargo test -p server_d --test cli_smoke` |
| [x] | Parité de reprise snapshot + WAL | `sy_infra/tests/recovery_determinism.rs` (v3/v4, redémarrage propre, rejeu après crash, proptest) | `cargo test -p sy_infra --test recovery_determinism` |
| [x] | Arrêt forcé & *burn-in* 100k | `bin/server_d/tests/forced_kill_recovery.rs`, `recovery_determinism.rs` (CI nocturne `ignored`) | `cargo test --workspace --all-targets -- --ignored` |
| [x] | Cibles fuzz WAL | [`crates/sy_infra/fuzz/README.md`](../../seej/server/crates/sy_infra/fuzz/README.md) | `cargo +nightly fuzz build` dans `crates/sy_infra/fuzz/` |
| [x] | `P2-01` `sy_loader` inerte retiré | Suppression des *placeholders* morts (`sy_loader`, `sy_tools`, `sy_protocol`, `mods/`, `examples/`) selon D-015 | `cargo metadata --no-deps` |

---

## Gate 1-K — Premier contrat stable noyau & plugin (3 jalons incrémentaux)

Chaque item non coché ci-dessous bloque le gel du contrat v1 (`K-01`). Le
travail est séquencé en trois jalons exécutables afin que chaque couche soit
testée et mesurée avant d'empiler la suivante.

### Jalon 1 — Cœur générique minimal & stockage local sûr face aux crashs

Résultat : un monde vide ou non spatial tourne localement sur le *narrow waist*
générique (`OrderedTable` + `ObligationRegistry` + unique `apply(WriteSet)` +
`EntropyScope`) avec un stockage durable *fail-closed* et des pannes de
stockage injectées.

- [ ] **K-06 : Propriété générique des données et transactions.**
  `OrderedTable` partitionné, `ObligationRegistry`, commandes publiques
  propriétaires, validateurs orientés deltas, transactions inter-propriétaires
  atomiques via `StateView` immuable + `CandidateOverlay`, recalcul complet sur
  base périmée et blocage du retrait d'un propriétaire en présence
  d'obligations actives.
- [ ] **K-03 : Fixtures contractuelles déterministes.** Fixtures attendues
  fixes pour un monde vide et une extension non spatiale (sans position, zone,
  créature ni ressource), plus une fixture de sémantique de lecture explicite
  $T-1$. Aucune assertion probabiliste.
- [ ] **P0-02 : Checkpoints liés aux ticks.** La cadence de checkpoint ne
  dépend que des ticks canoniques, jamais de l'horloge murale.
- [ ] **P0-03 : Décisions numériques déterministes.** Éliminer `f32`/`f64` des
  décisions canoniques ; API `EntropyScope` purement entière.
- [ ] **P0-04 : Compteurs canoniques *fail-closed*.** Arithmétique vérifiée sur
  les compteurs de tick, événement, séquence, incarnation, expiration, terme et
  configuration.
- [ ] **P0-05 : Persister le profil d'exécution.** L'histoire et les
  checkpoints lient l'intégralité du contrat affectant le rejeu, le mode de
  preuve, l'algorithme de hash, l'algorithme d'entropie et les limites.
- [ ] **P0-06 : Formaliser `GenesisSpec`.** Genèse versionnée et hachable
  consignant les domaines initiaux, contrats, politique d'aléa et profil
  d'autorité initial.
- [ ] **P0-07 : Découpler `world_id` de la graine.** Refuser toute incohérence
  monde/genèse/histoire ; les réplicas conservent `world_id`, les clones en
  allouent un nouveau.
- [ ] **P0-12 : Validateur générique d'intégrité du monde.** Validateur pur
  invoqué après décodage, reprise, migration et avant toute prétention
  d'autorité.
- [ ] **P0-13 : Séparer reprise de queue tronquée et corruption.** Modes
  explicites pour queue tronquée sûre, refus, quarantaine et réparation
  opérateur préservant les preuves forensiques.
- [ ] **P0-14 : Injection de pannes I/O courantes.** Injecter support plein /
  lecture seule, erreurs d'append, sync, renommage et lecture via les doubles
  de pannes `StateStore`/`AuthorityLog` ; prouver le comportement *fail-closed*
  sans aucun état non durable accepté.
- [ ] **P0-15 : Oracle de rejeu / re-simulation.** Recalculer les transitions
  conservées depuis un checkpoint ou la genèse + commandes + profil exact et
  comparer les racines sémantiques.
- [ ] **P1-04 : Séparer empreintes sémantiques, de stockage et d'autorité.**
  Racine sémantique incrémentale par enregistrement indépendante du layout de
  stockage physique.
- [ ] **P1-05 : Fixtures de compatibilité (*golden*).** Vecteurs explicites
  d'acceptation, migration et refus à travers les versions stables.
- [ ] **P1-06 : Versionner les variantes persistées.** Tags stables et refus
  typé pour les variantes obligatoires inconnues.
- [ ] **P1-07 : Séparer les DTO persistés des types Rust internes.**
- [ ] **P1-09 : Lier l'intégrité des checkpoints.** Détecter tout artefact de
  checkpoint, manifeste, log ou autorité tronqué, échangé, périmé ou incohérent.
- [ ] **P1-10 : Création durable de monde.** Tester les frontières de
  répertoire, manifeste, confirmation initiale et premier checkpoint.
- [ ] **P1-11 : Inspection locale sûre et non mutante.** Ouvrir un inspecteur
  ne crée jamais de répertoire ni ne mute de fichier, n'expose jamais de queue
  locale non confirmée et rejette tout chemin d'export qui résout dans le
  stockage canonique ou y fait alias.
- [ ] **P1-13 : Borner et valider les commandes externes avant exécution pure.**
- [ ] **P1-14 : Appliquer les limites de taille et de cardinalité.** Borner
  charges utiles, événements, transactions, checkpoints, messages, slots
  producteurs, générations non fermées, commandes en attente, *tombstones* et
  budgets réservés du plan de contrôle.
- [ ] **P1-15 : Persister le profil de limites.**
- [ ] **P1-16 : Extraire les règles applicatives du prototype.** Les règles de
  ressources/créatures ne tournent qu'en tant que plugin d'exemple/test hors de
  `sy_core`.
- [ ] **P1-18 : Retirer la réécriture automatique du WAL (`truncate_after`).**
- [ ] **P1-20 : Chaînage causal minimal et curseurs distincts.** Lier
  `pre_root`, `post_root`, `events_root`, hash de profil et curseurs distincts
  `confirmed_cursor`, `checkpoint_cursor` et `applied_cursor`.
- [ ] **P2-06 : Frontière des règles applicatives vérifiée en tests/CI.**
  (Politique consignée dans D-012/D-015 ; vérifier zéro ontologie applicative
  dans le nouveau cœur.)

### Jalon 2 — Autorité distribuée par monde & DST

Résultat : les déploiements à 1 nœud et 3 nœuds (quorum 2) obéissent au même
contrat exact, prouvé sous un harnais mono-thread de *Deterministic Simulation
Testing* (DST) ainsi que par de vrais tests multi-processus.

- [ ] **K-05 : Testabilité des coutures internes & conformité par injection de
  pannes (affiné par D-015).** Coutures internes propres (`StateStore`,
  `AuthorityLog`, `Executor`) vérifiées contre des doubles en mémoire et à
  injection de pannes couvrant la sémantique normale et de panne (remplace les
  neuf contrats publics de fournisseurs de D-014).
- [ ] **K-07 : Preuve d'autorité distribuée mono-monde.** Prouver le
  fonctionnement à 1 nœud et à 3 nœuds (quorum 2) : `TickCommit` group-committé,
  crash et remplacement du coordinateur, partition minoritaire en lecture seule,
  *fencing* sur terme/configuration périmés, rattrapage d'un *learner*,
  transition de membership conjointe (y compris interruption entre
  configuration conjointe et finale), redémarrage complet depuis terme/vote/config
  durables, barrières de lecture par quorum, refus sur profil incompatible et
  confinement des pannes de stockage local.
- [ ] **K-08 : Cycle de vie durable des commandes & voie de contrôle réservée.**
  Identité complète de commande, slot/incarnation producteur,
  `intention_fingerprint`, tick cible, fenêtre de statut positive, admission
  durable `Pending`, résolution terminale (`Succeeded`/`Refused`/`Cancelled`),
  `CancelCommand` sur son **budget de contrôle réservé** (jamais affamé par des
  quotas applicatifs pleins), blocage explicite de `CloseTick(T)` tant qu'une
  commande `Pending` cible `T`, course annulation-vs-exécution, expiration exacte
  vérifiée, générations confirmées `Open`/`Sealed`/`Closed`, retrait/réaffectation
  de slot et matérialisation du suffixe sans ré-exécution de commande.
- [ ] **P0-08 : Persister les enveloppes durables de commandes.**
- [ ] **P0-09 : Canoniser l'ordre des entrées concurrentes par monde.**
- [ ] **P0-10 : Appliquer un verrou d'écrivain local exclusif & le fencing
  d'autorité distribuée.**
- [ ] **P0-11 : Lier les enregistrements confirmés au monde, à la genèse, au
  profil, au terme, à la configuration et au curseur.**
- [ ] **P1-12 : Fixtures adverses de reprise et réplication** (doublon,
  manquant, réordonné, mauvais monde, mauvais profil, terme périmé, mauvais
  checkpoint).
- [ ] **P1-17 : Persister l'état de fencing à travers les redémarrages.**

### Jalon 3 — Observation, migrations, sauvegarde & extension externe → Gel v1

Résultat : des observateurs externes et un vrai plugin d'écriture tiers
consomment le contrat public, les données noyau migrent en sécurité avec des
checkpoints frontières obligatoires, et les opérateurs disposent de procédures
testées de sauvegarde/reprise.

- [ ] **K-02 : Surface d'observation confirmée.** Accès lecture seule à l'état
  et aux événements depuis un préfixe confirmé avec IDs d'événements stables,
  curseur, fraîcheur (confirmé courant via barrière de lecture par quorum vs
  explicitement périmé), limitation de débit, déduplication à la reconnexion,
  expiration de curseur et resynchronisation cohérente épinglée. Strictement
  neutre pour la simulation canonique.
- [ ] **K-04 : Véritable extension d'écriture externe.** Plugin maintenu
  indépendamment déclarant des données non spatiales, des commandes possédées
  et des validateurs, participant à une transaction inter-propriétaires,
  survivant à la reprise et n'important que le contrat public gelé (zéro
  dépendance sur les internes de `sy_core`).
- [ ] **K-09 : Compatibilité et migrations en 5 phases.** `MigrationPrepare`
  (`WorldPaused`), drainage/annulation complet des commandes `Pending` de
  l'ancien profil, `MigrationBarrier`, dérivation déterministe du candidat sur
  quorum, `MigrationActivate` avec **checkpoint frontière obligatoire**,
  abandon pré-activation, reprise post-activation, membership figé pendant la
  migration et refus des plugins incompatibles ou manquants.
- [ ] **K-10 & P1-19 : Compaction consciente de la rétention.** Rétention
  bornée préservant points de reprise, obligations, *tombstones* jusqu'à
  fermeture de génération, planchers de générations fermées par incarnation,
  marques hautes d'incarnation par slot et resynchronisation des observateurs
  sous rotation continue des producteurs.
- [ ] **P2-02 : Procédure testée de sauvegarde et restauration à froid.**
- [ ] **P2-03 : Runbook opérateur de reprise de base** (corruption, erreur de
  stockage, migration échouée, perte de quorum, nœud périmé, plugin manquant).
- [ ] **K-01 : Geler le contrat public v1.** Geler `CONTRACT.md`,
  `PLUGIN_CONTRACT.md` et les schémas publics uniquement après réussite des
  Jalons 1 à 3 et vérification de chaque commande, lien, ancre et test de la
  matrice de pannes.

---

## Gate 1-D — Durcissement et passage à l'échelle tirés par l'usage (post-v1)

Différés jusqu'à déclenchement par des charges ou mesures réelles ; aucun ne
bloque le gel v1, et tous sont additifs au-dessus du contrat plugin v1 gelé :

### Déclencheur : charges massives denses (ex. Charge B — $10^5$ écritures/tick)
- [ ] **Checkpoints CoW asynchrones hors chemin critique** (éliminer le gel
  synchrone de 240–435 ms mesuré pour 144 Mo dans le harnais de laboratoire).
- [ ] **Mode de preuve compact vérifié par tick (`root`)** dans les profils
  d'exécution une fois le déterminisme plugin garanti par isolation ou profils
  vérifiés.
- [ ] **`Executor` parallèle multi-cœur** (`Block-STM` + `BoundedDelta`
  optionnel pour les transactions K-06 ; `BSP` parallèle sur partitions pour
  les passes $T-1$ déclarées), vérifié contre l'oracle de référence
  mono-thread.
- [ ] **Backends de stockage à fort débit d'écriture & `TableKind::Columnar`**
  (ex. LSM / tables colonnaires par blocs) derrière la couture interne
  `StateStore`.

### Déclencheur : plugins non fiables / dynamiques (Évolution 2)
- [ ] **Exécution isolée de plugins (WASM)** avec comptage de fuel déterministe,
  canonisation des NaN et transport par tampons mémoire plats en batch.
- [ ] **Installation à chaud de plugins, mise en pause, remplacement et
  transfert d'obligations.**

### Déclencheur : passage à l'échelle P2P multi-mondes
- [ ] **Protocole de fédération inter-mondes :** enveloppes authentifiées
  asynchrones et obligations inter-mondes entre mondes CP indépendants.

### Déclencheur : multi-plateforme / opérateurs externes / diagnostics
- [ ] **P1-03 :** Fixtures de parité des racines sémantiques multi-plateformes
  (Linux, Windows, macOS).
- [ ] **P1-08 :** Politique de persistance numérique portable si une charge
  utile flottante non canonique subsiste.
- [ ] **P1-21 :** Enregistreur de vol déterministe (*flight recorder*).
- [ ] **P1-22 :** Commande de vérification lecture seule `doctor`.
- [ ] **P1-23 :** Bisecteur de divergence à travers les histoires conservées.
- [ ] **P2-04 :** Diagnostics structurés de reprise.
- [ ] **P2-05 :** Matrice étendue chaos/pannes au-delà des scénarios de base.
- [ ] **P0-16 :** Matrice élargie de points de crash/chaos sur toutes les
  frontières.

---

## Écosystème communautaire optionnel

Jamais des obligations de livraison du noyau Seej ; les communautés construisent
ces composants indépendamment au-dessus des contrats publics gelés v1 :

- Modèles spatiaux (grilles 2D/3D, octrees, graphes) et modèles de mondes non
  spatiaux ;
- Économies, écologies, physique, narrations, agents et intégrations LLM ;
- Moteurs d'agrégation multirésolution / résolution causale ;
- Moteurs de rendu 2D/3D/web/Unreal/Godot et tableaux de bord d'administration ;
- Netcode joueur ingame (prédiction, interpolation, sessions clients).
