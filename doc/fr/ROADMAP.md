# Feuille de route et plan de codage ordonné

> **Traduction non normative** de [`doc/ROADMAP.md`](../ROADMAP.md). La version
> anglaise fait foi en cas d'écart.

Ce document est l'autorité unique pour le séquencement du projet, les phases de
codage ordonnées, le statut d'implémentation et les preuves de sortie. Les
contrats de conception dans `seej/docs/` (et leurs miroirs ici) décrivent des
cibles et ne ferment jamais un item de checklist.

## Convention de statut et discipline

- `[x]` — preuve reproductible commitée dans ce dépôt.
- `[ ]` — cible ouverte, quel que soit le niveau de détail de sa spécification.
- Les identifiants d'écarts stables (`K-*`, `P0-*`, `P1-*`, `P2-*`) sont
  préservés à travers les réorganisations ; les items retirés sont signalés
  explicitement avec leur identifiant de décision.
- **Discipline séquentielle stricte ([D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée)) :**
  terminer et vérifier chaque phase dans l'ordre (`Phase 1.1` → `1.2` → `1.3`
  → `1.4` → `1.5`, puis `Phase 2`, puis `Phase 3`). Ne pas ajouter
  d'abstractions spéculatives, de crates supplémentaires ni d'optimisations de
  Phase 4 avant leur phase.
- **Documentation dans le code :** chaque crate dans `seej/server/crates/`
  impose `#![deny(missing_docs)]` et embarque une Rustdoc de niveau crate
  alignée sur sa phase exacte ci-dessous
  (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`).

---

## Phase 0 — Socle vierge et garde-fous (terminée)

Le prototype spatial historique (`Entity`/`Zone`, snapshots JSON, double chemin
`sim.rs`/`replay.rs`, fixtures `server_d`/`sy_cli`) a été retiré selon D-015.
Le dépôt repart désormais d'un squelette workspace minimal en 4 crates
(`sy_types`, `sy_api`, `sy_core`, `sy_infra`) avec ses garde-fous actifs :

| Fait | Critère | Preuve | Reproduire depuis la racine du dépôt |
| --- | --- | --- | --- |
| [x] | `P0-01` porte de pureté | [`check_sy_core_purity.py`](../../seej/server/scripts/check_sy_core_purity.py) (18 auto-tests sur `sy_types`/`sy_api`/`sy_core` dont `f32`/`f64`, `std::thread`, littéraux `char`) + [`check_dag.py`](../../seej/server/scripts/check_dag.py) (normal + dev + build) | `python3 -B seej/server/scripts/check_sy_core_purity.py --self-test` && `python3 -B seej/server/scripts/check_sy_core_purity.py` && `python3 -B seej/server/scripts/check_dag.py` |
| [x] | `P1-01` CI du dépôt & check unifié | [`check.py`](../../seej/server/scripts/check.py) + [`check_docs.py`](../../seej/server/scripts/check_docs.py) + [`.github/workflows/rust.yml`](../../.github/workflows/rust.yml) (`--locked`, `deny` incl. `bans`, CI PR open-source) | `python3 -B seej/server/scripts/check.py` |
| [x] | `P1-02` chaîne épinglée | [`rust-toolchain.toml`](../../rust-toolchain.toml) & [`seej/server/rust-toolchain.toml`](../../seej/server/rust-toolchain.toml) (`1.95.0`) + `rust-version = "1.95.0"` | `rustc --version` |
| [x] | `P2-01` *placeholders* inertes retirés | Suppression des *placeholders* morts (`sy_loader`, `sy_tools`, `sy_protocol`, `mods/`, `examples/`) et prototype archivé au tag `archive/prototype-v1-final` (`5841a00`) | `git show --stat archive/prototype-v1-final` |
| [x] | Référence D-015 & benchmarks empiriques | [DECISIONS.md](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée) & [BENCHMARKS.md](BENCHMARKS.md) ; harnais bruts épinglés à `archive/lab-campaign-1` (`1c98282`) & `archive/lab-campaign-2` (`5856396`) | `git show archive/lab-campaign-1:seej/server/lab/workloads/RESULTS.md` & `git show archive/lab-campaign-2:seej/server/lab/workloads/RESULTS-layout.md` |

---

## Gate 1-K — Premier contrat stable noyau & plugin (Phases 1–3)

Chaque item non coché ci-dessous bloque le gel du contrat v1 (`K-01`).

### Phase 1 — Cœur générique minimal & stockage local sûr face aux crashs

Résultat : un monde vide ou non spatial tourne localement sur le *narrow waist*
générique (`OrderedTable` + `ObligationRegistry` + unique `apply(WriteSet)` +
`EntropyScope`) avec un stockage durable *fail-closed* et des pannes de
stockage injectées.

#### Phase 1.1 — Primitives stables, compteurs vérifiés & types de profils (`sy_types`)

- [ ] **P0-04 : Compteurs canoniques *fail-closed*.** Arithmétique vérifiée sur
  `Tick`, `EventId`, `AdmissionSeq`, `AuthorityTerm`, `ConfigVersion`,
  incarnation producteur, génération et compteurs d'expiration (jamais de
  bouclage ni de saturation).
- [ ] **P0-03 : Discipline numérique déterministe.** Éliminer `f32`/`f64` des
  décisions canoniques ; primitives purement entières / à virgule fixe.
- [ ] **P0-07 : Découpler `world_id` de la graine.** `WorldId` et `NodeId`
  indépendants ; refuser toute incohérence monde/genèse/histoire ; les réplicas
  conservent `world_id`, les clones en allouent un nouveau.
- [ ] **P0-06 : Formaliser `GenesisSpec`.** Genèse versionnée et hachable
  consignant les domaines initiaux, contrats, politique d'aléa et profil
  d'autorité initial.
- [ ] **P0-05 : Persister le profil d'exécution.** `ExecutionProfile` fortement
  typé liant versions de contrats, `ProofMode` (`delta` par défaut v1, futur
  `root`), algorithme de hash, algorithme d'entropie, empreintes des plugins
  actifs et politique de rétention.
- [ ] **P1-15 : Persister le profil de limites.** `LimitsProfile` fini incluant
  le budget isolé réservé au plan de contrôle pour `CancelCommand` et le
  contrôle de migration.
- [ ] **P1-06 : Versionner les variantes persistées.** Tags stables et refus
  typé pour les variantes obligatoires inconnues.
- [ ] **P1-07 : Séparer les DTO persistés des types Rust internes.**

#### Phase 1.2 — Couture publique plugin gelée & traits internes (`sy_api`)

- [ ] **État logique public & contrat plugin en batch plat (socle `K-06`) :**
  `OrderedTable` partitionné `(DomainId, TableId, PartitionKey, RecordKey) ->
  OpaqueBytes`, `TableKind` extensible, `ObligationRegistry`, `StateView`
  immuable, `WriteSet`, `PluginManifest`, point d'entrée batch pur
  `(StateView, InputBatch, EntropyScope, Context) -> Decision + WriteSet +
  EventBatch + OutboxIntents`, sémantique de lecture explicite `Sequential` vs
  `SnapshotTMinus1`, `AccessHint` optionnel et traits de validateurs
  inter-domaines orientés deltas.
- [ ] **Types publics de commandes et résolutions :** `CommandEnvelope`,
  `Admission`, `Resolution` (`Succeeded`, `Refused`, `Cancelled`),
  `CancelCommand`, `CloseTick` et retours de recherche/transitoires (`Conflict`,
  `Expired`, `GenerationSealed`, `LimitExceeded`, `NotFound`, `Unavailable`,
  `WorldPaused`).
- [ ] **Coutures d'infrastructure internes (socle `K-05`) :** traits Rust
  internes propres (non gelés publiquement) `StateStore`, `AuthorityLog` et
  `Executor`.

#### Phase 1.3 — Moteur de transition pur & chemin de mutation unique (`sy_core`)

- [ ] **K-06 : Propriété générique des données et transactions.** Unique
  mutateur canonique `apply(WriteSet)` (zéro `world.clone()`, zéro double
  logique `sim`/`replay`), `CandidateOverlay`, transactions
  inter-propriétaires atomiques, recalcul complet sur base périmée, zéro
  `Refused` fabriqué sur panique/échec plugin et blocage du retrait d'un
  propriétaire en présence d'obligations actives.
- [ ] **`EntropyScope` dérivé :** PRF/PRNG entier à compteur dérivé de
  coordonnées canoniques avec `.fork(stable_key)` et IDs dérivés de la cause.
- [ ] **P0-12 : Validateur générique d'intégrité du monde.** Validateur pur
  invoqué après décodage, reprise, migration et avant toute prétention
  d'autorité.
- [ ] **P1-04 : Séparer empreintes sémantiques, de stockage et d'autorité.**
  Racine sémantique incrémentale par enregistrement indépendante du layout de
  stockage physique.
- [ ] **P1-20 : Chaînage causal minimal et curseurs distincts.** Lier
  `pre_root`, `post_root`, `events_root`, hash de profil et curseurs distincts
  `confirmed_cursor`, `checkpoint_cursor` et `applied_cursor`.
- [ ] **P1-13 : Borner et valider les commandes externes avant exécution pure.**
- [ ] **P1-14 : Appliquer les limites de taille et de cardinalité.** Borner
  charges utiles, événements, transactions, checkpoints, messages, slots
  producteurs, générations non fermées, commandes en attente, *tombstones* et
  budgets réservés du plan de contrôle.
- [ ] **K-03 : Fixtures contractuelles déterministes.** Fixtures attendues
  fixes pour un monde vide, une extension non spatiale (sans position, zone,
  créature ni ressource), les transactions inter-propriétaires, le blocage
  d'obligations et une passe autonome $T-1$ explicite prouvant l'indépendance à
  l'ordre des partitions.
- [ ] **P2-06 : Frontière des règles applicatives vérifiée en tests/CI.**
  Vérifier zéro ontologie applicative dans `sy_core` et zéro import de
  `sy_core` par les plugins.

#### Phase 1.4 — `StateStore` local durable, WAL *fail-closed* & checkpoints (`sy_infra`)

- [ ] **`StateStore` local partitionné :** benchmarker les moteurs candidats
  (`redb`, `Fjall` ou store paginé partitionné maison) sur des charges réelles
  et implémenter `apply(WriteSet)` atomique + `pin_cut`.
- [ ] **P0-13 : Séparer la reprise de queue tronquée de la corruption.** WAL
  binaire *fail-closed* avec modes explicites pour queue tronquée sûre
  (préfixe d'en-tête court), refus, quarantaine et réparation opérateur
  préservant les preuves forensiques, plus de nouvelles cibles `cargo-fuzz` de
  décodage/aller-retour réintégrées en CI.
- [ ] **P1-18 : Zéro réécriture automatique du WAL.** Ne jamais tronquer
  au-delà d'enregistrements valides ni réécrire silencieusement l'histoire
  confirmée.
- [ ] **P0-02 : Checkpoints liés aux ticks.** La cadence de checkpoint ne
  dépend que des ticks canoniques, jamais de l'horloge murale.
- [ ] **P1-09 : Lier l'intégrité des checkpoints.** Détecter tout artefact de
  checkpoint, manifeste, log ou autorité tronqué, échangé, périmé ou incohérent.
- [ ] **P1-10 : Création durable de monde.** Tester les frontières de
  répertoire, manifeste, confirmation initiale et premier checkpoint.
- [ ] **P0-14 : Injection de pannes I/O courantes.** Injecter support plein /
  lecture seule, erreurs d'append, sync, renommage et lecture via les doubles
  de pannes `StateStore`/`AuthorityLog` ; prouver le comportement *fail-closed*
  sans aucun état non durable accepté.

#### Phase 1.5 — Oracle de rejeu, vecteurs *golden* & CLI locale sûre (`sy_infra`, `server_d`, `sy_cli`)

- [ ] **P0-15 : Oracle de rejeu / re-simulation.** Recalculer les transitions
  conservées depuis un checkpoint ou la genèse + commandes + profil exact et
  comparer les racines sémantiques.
- [ ] **P1-05 : Fixtures de compatibilité (*golden*).** Vecteurs explicites
  d'acceptation, migration et refus à travers les versions stables.
- [ ] **P1-11 : Inspection locale sûre et non mutante.** Ouvrir un inspecteur
  ne crée jamais de répertoire ni ne mute de fichier, n'expose jamais de queue
  locale non confirmée et rejette tout chemin d'export qui résout dans le
  stockage canonique ou y fait alias.
- [ ] **P1-16 : Plugin applicatif d'exemple hors de `sy_core`.** Plugin de
  domaine de référence/exemple optionnel démontrant l'usage du contrat public
  sans toucher aux internes de `sy_core`.

---

### Phase 2 — Autorité distribuée par monde & DST

Résultat : les déploiements à 1 nœud et 3 nœuds (quorum 2) obéissent au même
contrat exact, prouvé sous un harnais mono-thread de *Deterministic Simulation
Testing* (DST) ainsi que par de vrais tests multi-processus.

#### Phase 2.1 — Doubles de coutures internes & DST mono-thread

- [ ] **K-05 : Testabilité des coutures internes & conformité par injection de
  pannes (affiné par D-015).** Coutures internes propres (`StateStore`,
  `AuthorityLog`, `Executor`) vérifiées contre des doubles en mémoire et à
  injection de pannes couvrant la sémantique normale et de panne (remplace les
  neuf contrats publics de fournisseurs de D-014).

#### Phase 2.2 — Cycle de vie durable des commandes & plan de contrôle réservé

- [ ] **P0-08 : Persister les enveloppes durables de commandes.**
- [ ] **P0-09 : Canoniser l'ordre des entrées concurrentes par monde.**
- [ ] **K-08 : Cycle de vie durable des commandes & voie de contrôle réservée.**
  Identité complète de commande, slot/incarnation producteur,
  `intention_fingerprint`, tick cible, fenêtre de statut positive, admission
  durable `Pending`, résolution terminale (`Succeeded`/`Refused`/`Cancelled`),
  `CancelCommand` sur son **budget de contrôle réservé** (jamais affamé par des
  quotas applicatifs pleins), blocage explicite de `CloseTick(T)` tant qu'une
  commande `Pending` cible `T`, course annulation-vs-exécution, expiration exacte
  vérifiée, générations confirmées `Open`/`Sealed`/`Closed`, retrait/réaffectation
  de slot et matérialisation du suffixe sans ré-exécution de commande.

#### Phase 2.3 — Autorité mono-monde, fencing & membership conjoint

- [ ] **P0-10 : Appliquer un verrou d'écrivain local exclusif & le fencing
  d'autorité distribuée.**
- [ ] **P0-11 : Lier les enregistrements confirmés au monde, à la genèse, au
  profil, au terme, à la configuration et au curseur.**
- [ ] **P1-17 : Persister l'état de fencing à travers les redémarrages.**
- [ ] **K-07 : Preuve d'autorité distribuée mono-monde.** Prouver le
  fonctionnement à 1 nœud et à 3 nœuds (quorum 2) : `TickCommit` group-committé,
  crash et remplacement du coordinateur, partition minoritaire en lecture seule,
  *fencing* sur terme/configuration périmés, rattrapage d'un *learner*,
  transition de membership conjointe (y compris interruption entre
  configuration conjointe et finale), redémarrage complet depuis terme/vote/config
  durables, barrières de lecture par quorum, refus sur profil incompatible et
  confinement des pannes de stockage local.

#### Phase 2.4 — Réplication adverse & preuve multi-processus

- [ ] **P1-12 : Fixtures adverses de reprise et réplication** (doublon,
  manquant, réordonné, mauvais monde, mauvais profil, terme périmé, mauvais
  checkpoint) vérifiées à la fois sous DST mono-thread et en vrais tests de
  crash multi-processus.

---

### Phase 3 — Observation, migrations, compaction & extension externe → Gel v1

Résultat : des observateurs externes et un vrai plugin d'écriture tiers
consomment le contrat public, les données noyau migrent en sécurité avec des
checkpoints frontières obligatoires, et les opérateurs disposent de procédures
testées de sauvegarde/reprise.

#### Phase 3.1 — Surface d'observation confirmée

- [ ] **K-02 : Surface d'observation confirmée.** Accès lecture seule à l'état
  et aux événements depuis un préfixe confirmé avec IDs d'événements stables,
  curseur, fraîcheur (confirmé courant via barrière de lecture par quorum vs
  explicitement périmé), limitation de débit, déduplication à la reconnexion,
  expiration de curseur et resynchronisation cohérente épinglée. Strictement
  neutre pour la simulation canonique.

#### Phase 3.2 — Migrations coordonnées en 5 phases

- [ ] **K-09 : Compatibilité et migrations en 5 phases.** `MigrationPrepare`
  (`WorldPaused`), drainage/annulation complet des commandes `Pending` de
  l'ancien profil, `MigrationBarrier`, dérivation déterministe du candidat sur
  quorum, `MigrationActivate` avec **checkpoint frontière obligatoire**,
  abandon pré-activation, reprise post-activation, membership figé pendant la
  migration et refus des plugins incompatibles ou manquants.

#### Phase 3.3 — Compaction consciente de la rétention

- [ ] **K-10 & P1-19 : Compaction consciente de la rétention.** Rétention
  bornée préservant points de reprise, obligations, *tombstones* jusqu'à
  fermeture de génération, planchers de générations fermées par incarnation,
  marques hautes d'incarnation par slot et resynchronisation des observateurs
  sous rotation continue des producteurs.

#### Phase 3.4 — Extension d'écriture externe & procédures opérateur

- [ ] **K-04 : Véritable extension d'écriture externe.** Plugin maintenu
  indépendamment déclarant des données non spatiales, des commandes possédées
  et des validateurs, participant à une transaction inter-propriétaires,
  survivant à la reprise et n'important que le contrat public gelé (zéro
  dépendance sur les internes de `sy_core`).
- [ ] **P2-02 : Procédure testée de sauvegarde et restauration à froid.**
- [ ] **P2-03 : Runbook opérateur de reprise de base** (corruption, erreur de
  stockage, migration échouée, perte de quorum, nœud périmé, plugin manquant).

#### Phase 3.5 — Gel du contrat public v1

- [ ] **K-01 : Geler le contrat public v1.** Geler `CONTRACT.md`,
  `PLUGIN_CONTRACT.md` et les schémas publics uniquement après réussite des
  Phases 1 à 3 et vérification de chaque commande, lien, ancre et test de la
  matrice de pannes.

---

## Phase 4 — Passage à l'échelle, isolation & fédération post-v1 (Gate 1-D, tiré par l'usage)

Différés jusqu'à déclenchement par des charges ou mesures réelles ; aucun ne
bloque le gel v1, et tous sont additifs au-dessus du contrat plugin v1 gelé :

### Phase 4.1 — Passage à l'échelle en charge dense (ex. Charge B — $10^5$ écritures/tick)
- [ ] **Checkpoints CoW asynchrones hors chemin critique** (éliminer le gel
  synchrone de 237–435 ms — pointe à 1 485 ms — mesuré pour 144 Mo dans [BENCHMARKS.md](BENCHMARKS.md)).
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

### Phase 4.2 — Plugins non fiables / dynamiques (Évolution 2)
- [ ] **Exécution isolée de plugins (WASM)** avec comptage de fuel déterministe,
  canonisation des NaN et transport par tampons mémoire plats en batch.
- [ ] **Installation à chaud de plugins, mise en pause, remplacement et
  transfert d'obligations.**

### Phase 4.3 — Fédération P2P multi-mondes
- [ ] **Protocole de fédération inter-mondes :** enveloppes authentifiées
  asynchrones et obligations inter-mondes entre mondes CP indépendants.

### Phase 4.4 — Parité multi-plateforme & diagnostics
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
