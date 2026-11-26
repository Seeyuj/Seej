# Journal des décisions

> **Traduction non normative** de [`doc/DECISIONS.md`](../DECISIONS.md). La
> version anglaise fait foi en cas d'écart.

Ce document consigne les décisions architecturales structurantes de Seej : ce
qui a été décidé, pourquoi, ce que cela implique et ce que les décisions
ultérieures ont remplacé. Ce n'est ni une feuille de route (voir
[ROADMAP.md](ROADMAP.md)) ni un contrat (voir [CONTRACT.md](CONTRACT.md) et
[PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md)).

## Index des décisions

> **Note de traçabilité :** les entrées `D-001` à `D-014` ci-dessous ont été
> condensées en résumés concis par `D-015` sans modifier leur intention
> acceptée. Leur texte intégral pré-D-015 est archivé au tag d'archive
> `archive/pre-d015-docs` (`9662a60`).

| ID | Décision | Statut |
| --- | --- | --- |
| [D-001](#d-001--le-projet-est-une-plateforme-pas-un-jeu) | Une plateforme, pas un jeu | Acceptée |
| [D-002](#d-002--politique-narrative-hors-du-noyau) | Politique narrative hors du noyau | Acceptée (recentrée par D-012) |
| [D-003](#d-003--monde-autonome-non-centré-sur-le-joueur) | Monde autonome sans joueur | Acceptée (recentrée par D-012) |
| [D-004](#d-004--serveur-autoritaire-et-persistance-réelle) | Groupe d'autorité et persistance réelle | Acceptée (étendue par D-012) |
| [D-005](#d-005--découplage-strict-entre-simulation-et-rendu) | Découplage strict vis-à-vis du rendu | Acceptée |
| [D-006](#d-006--client-de-rendu-de-référence-et-standard-graphique-officiel) | Client de référence Unreal officiel | Remplacée par D-012 |
| [D-007](#d-007--ia-pragmatique-déterministe-et-explicable) | Entrées externes déterministes validées | Acceptée (recentrée par D-012) |
| [D-008](#d-008--cœur-minimal-extensions-modulaires) | Cœur minimal, extensions modulaires | Acceptée (affinée par D-012, D-015) |
| [D-009](#d-009--stabilité-et-maintenabilité-avant-la-vitesse) | Stabilité et maintenabilité avant la vitesse | Acceptée |
| [D-010](#d-010--rust-pour-le-cœur) | Rust pour le cœur pur | Acceptée |
| [D-011](#d-011--noyau-orienté-contrat-durcissement-tiré-par-lusage) | Noyau orienté contrat, durcissement tiré par l'usage | Remplacée par D-012 (discipline incrémentale restaurée par D-015) |
| [D-012](#d-012--noyau-de-monde-distribué-et-extensible) | Noyau de monde distribué et extensible | Acceptée (amendée par D-013, D-014, D-015) |
| [D-013](#d-013--sémantique-opérationnelle-des-pannes) | Sémantique opérationnelle des pannes | Acceptée (amendée par D-014, D-015) |
| [D-014](#d-014--sémantique-des-frontières-de-la-première-version-stable) | Sémantique des frontières de la première version stable | Acceptée (amendée par D-015) |
| [D-015](#d-015--noyau-mesuré-et-couture-plugin-gelée) | Noyau mesuré et couture plugin gelée | **Acceptée (référence actuelle)** |

---

## D-001 — Le projet est une plateforme, pas un jeu

**Statut :** Acceptée

Seej est une infrastructure open-source pour des mondes bac à sable
déterministes et persistants — pas un jeu, ni un moteur graphique, ni un RPG
narratif, ni un framework de gameplay. Sa valeur réside dans la stabilité du
cœur de simulation, une persistance réelle, la cohérence systémique et la
maintenabilité sur plusieurs années. Les clients restent des consommateurs,
jamais des piliers architecturaux.

---

## D-002 — Politique narrative hors du noyau

**Statut :** Acceptée, portée amendée par D-012

Le noyau définit les mécaniques déterministes de monde et ne prescrit aucun
modèle narratif. Les extensions peuvent implémenter une narration écrite,
procédurale ou émergente, soumise aux mêmes règles d'admission, de validation,
de propriété et de persistance que toute autre capacité applicative.

---

## D-003 — Monde autonome non centré sur le joueur

**Statut :** Acceptée, portée amendée par D-012

Un monde évolue et persiste sans aucun joueur connecté ni présence humaine. Le
noyau n'intègre aucune catégorie « joueur » ni aucun privilège humain ; les
extensions peuvent déclarer explicitement des rôles applicatifs.

---

## D-004 — Serveur autoritaire et persistance réelle

**Statut :** Acceptée, étendue aux groupes d'autorité par D-012

Un groupe de nœuds explicitement autorisé est l'unique autorité canonique sur
l'histoire d'un monde. Les déploiements mono-nœud et multi-nœuds suivent le
même contrat public. Les clients ne détiennent aucune autorité canonique.

---

## D-005 — Découplage strict entre simulation et rendu

**Statut :** Acceptée

Le cœur de simulation est totalement indépendant des technologies de rendu et
des moteurs clients. Aucune bibliothèque graphique ni aucun framework client
n'est une dépendance du noyau ou du runtime.

---

## D-006 — Client de rendu de référence et standard graphique officiel

**Statut :** Remplacée par D-012

Direction historique promettant un client de référence officiel sous Unreal
Engine. Remplacée par D-012 : les moteurs de rendu (Unreal, Godot, web, CLI)
sont des consommateurs communautaires optionnels des contrats publics, pas des
livrables du noyau Seej.

---

## D-007 — IA pragmatique, déterministe et explicable

**Statut :** Acceptée pour les entrées du noyau, portée amendée par D-012

Le noyau n'accepte que des entrées bornées, validées, ordonnées et persistées,
et ne livre aucun framework d'agents intégré. Les systèmes externes non
déterministes (LLM, capteurs, bots) n'influencent l'état canonique qu'en
soumettant des commandes validées.

---

## D-008 — Cœur minimal, extensions modulaires

**Statut :** Acceptée, étendue par D-012 et affinée par D-015

Garder le cœur minimal, strict et stable. Les capacités applicatives sont des
extensions composées au-dessus du contrat public du noyau plutôt qu'accumulées
dans un monolithe.

---

## D-009 — Stabilité et maintenabilité avant la vitesse

**Statut :** Acceptée

La durabilité architecturale, l'exactitude, le déterminisme et la
maintenabilité priment sur l'optimisation prématurée ou les fonctionnalités
précipitées. Les choix de performance doivent préserver la sémantique et
s'appuyer sur des mesures.

---

## D-010 — Rust pour le cœur

**Statut :** Acceptée

Le cœur de simulation est écrit en Rust pour la sûreté mémoire, les
performances natives prévisibles, le typage fort et les frontières explicites
de revue `unsafe`/FFI. Les consommateurs et les futurs plugins isolés ne sont
pas limités à Rust.

---

## D-011 — Noyau orienté contrat, durcissement tiré par l'usage

**Statut :** Remplacée par D-012 (discipline incrémentale restaurée par D-015)

A positionné Seej comme un noyau de monde orienté contrat en découpant la
sortie en Gate 1-K (contrat gelé) et Gate 1-D (durcissement tiré par l'usage).
D-012 a remplacé sa portée mono-écrivain et son hypothèse de conservation
intégrée, tout en conservant le principe orienté contrat et la séparation
Gate 1-K / Gate 1-D.

---

## D-012 — Noyau de monde distribué et extensible

**Statut :** Acceptée, sémantique opérationnelle amendée par D-013, D-014 et D-015

A défini Seej comme un noyau logique de monde distribué sans coordinateur
global :

- Chaque monde possède un groupe de nœuds explicitement autorisé (profil de
  référence : trois nœuds *shared-nothing*, quorum deux, consensus tolérant aux
  pannes franches de type Raft ; le consensus byzantin est hors périmètre v1).
  La découverte n'accorde jamais l'appartenance au groupe.
- `world_id`, `node_id`, la graine (*seed*) et `genesis_hash` sont distincts.
  Les ticks et l'ordre des commandes sont canoniques par monde, sans horloge
  globale entre mondes.
- Les données d'un monde appartiennent à des domaines propriétaires déclarés,
  sans espace obligatoire, ni entités, ni zones, ni lois économiques intégrées.
- L'activation au démarrage ne charge en v1 que des composants de confiance
  approuvés par l'opérateur ; le chargement à chaud et l'exécution isolée de
  plugins relèvent d'une étape ultérieure.
- L'observation lit un préfixe confirmé accompagné de sa fraîcheur et n'altère
  jamais la simulation canonique.
- Répliquer un monde améliore la disponibilité et la capacité de reprise ; cela
  ne distribue pas le calcul d'un tick sur plusieurs machines.

---

## D-013 — Sémantique opérationnelle des pannes

**Statut :** Acceptée, amendée par D-014 et D-015

A fermé les principales ambiguïtés de frontières de pannes de D-012 :

- Séparation entre **Admission** confirmée (`Pending`, sans effet applicatif)
  et **Résolution** confirmée (`Succeeded`, `Refused` ou `Cancelled`).
- Exigence d'expiration sur tick canonique vérifié et de non-réutilisation des
  identifiants de commandes expirées.
- Définition des transactions inter-propriétaires sur un curseur de base
  immuable avec validation privée d'un candidat et recalcul complet en cas de
  base périmée ; un crash de composant ne fabrique jamais de décision `Refused`
  déterministe.
- Exigence de transitions de membership par configuration conjointe préservant
  l'intersection des quorums ancien/nouveau, et de barrières de lecture
  appuyées sur le quorum pour les lectures confirmées courantes.
- Séparation entre préparation de migration, commit d'activation et abandon
  (aucun rollback automatique post-activation).
- Rend obligatoire avant le gel v1 le chaînage causal par empreinte, la
  sauvegarde/restauration à froid et les procédures opérateur de reprise de
  base.

---

## D-014 — Sémantique des frontières de la première version stable

**Statut :** Acceptée, amendée par D-015

A ajouté des mécanismes de frontière concrets à D-013 :

- Définition de `CancelCommand` autorisé et confirmé, en course avec
  l'exécution normale via l'ordre confirmé, comme voie de déblocage
  administrative pour les commandes `Pending` bloquées.
- Ajout d'un `MigrationBarrier` post-drainage gelant les ticks et les
  admissions applicatives (`WorldPaused`) sous une configuration de membership
  fixe avant dérivation déterministe du candidat et `MigrationActivate`.
- Définition de slots producteurs bornés par monde avec incarnations
  monotones, générations confirmées `Open` / `Sealed` / `Closed`, pierres
  tombales (*tombstones*), planchers de générations fermées et limites
  persistées finies.
- Distinction entre `confirmed_cursor`, `checkpoint_cursor` et `applied_cursor`.
- Clarification des écarts lecture seule du prototype retiré `sy_cli`
  (création de répertoire par `FilesystemStore::new` et aliasing non gardé de
  `dump --output`) sous `P1-11`, conservés comme critères d'acceptation pour
  la future CLI.

---

## D-015 — Noyau mesuré et couture plugin gelée

**Statut :** Acceptée

### Contexte et preuves empiriques

Un audit de l'historique du dépôt et du code après D-014 a révélé trois
problèmes structurels :

1. **Spécification en avance sur l'exécution :** 29 commits consécutifs ont
   édité la documentation et la CI sans faire avancer le cœur Rust ; Gate 1-K
   comptait 0 case cochée et zéro benchmark. Pendant ce temps, le prototype
   local (`seej/server/`) clonait le `World` entier deux fois par commande et
   une fois par événement rejoué, dupliquait la logique de mutation entre
   `sim.rs` et `replay.rs`, et portait des crates *placeholders* morts.
2. **Coutures publiques sur-fragmentées et hypothèses non mesurées :** exiger
   neuf contrats de fournisseurs publics gelés séparément (`PLUGIN_CONTRACT.md`
   §8 dans D-014) créait une fragmentation micro-noyau sur des composants
   partageant une même frontière de commit atomique (log, consensus,
   membership, fencing), tout en spécifiant des mécanismes lourds de
   persistance/hachage avant de les mesurer.
3. **Faille de vivacité (*liveness*) dans `CONTRACT.md` :** `CancelCommand`
   partageait les quotas d'admission applicatifs ordinaires ; un monde bloqué
   sur une commande `Pending` avec des quotas saturés pouvait donc rejeter
   `CancelCommand` avec `LimitExceeded` alors que les ticks ne pouvaient plus
   avancer pour faire expirer les anciens enregistrements.

Deux campagnes de benchmarks jetables mono-thread conservées aux tags d'archive
`archive/lab-campaign-1` (`1c98282`, 6/6 reprises propres) et
`archive/lab-campaign-2` (`5856396`, 24/24 reprises propres — cache OS non
purgé, température effective du cache inconnue, sans injection de pannes), sur
Ryzen 9 5900X + NVMe Crucial P2 + Rust 1.95.0, 1 000 000 clés × 128 o, budget
tick 10 Hz / 100 ms, ont établi cinq faits concrets (tableaux complets dans
[BENCHMARKS.md](BENCHMARKS.md)) :

- **Les charges de commandes et mixtes (A : 1 000 cmd/tick ; C : 1 000 cmd +
  1 000 écritures autonomes + 80 % de clés chaudes)** tiennent le budget de
  100 ms sur un thread en mode `delta` sur toutes les variantes hors rehash :
  **p99 = 8,62–21,74 ms** (`A` : `8,62–10,83 ms`, `C` : `11,77–21,74 ms`,
  médiane `6,64–9,95 ms` ; **marge 4,6×–11,6×**), journal **4,17–5,61 Mo/s**,
  rejeu propre **2,06–3,35 ms/tick**.
- **La charge autonome dense (B : 100 000 écritures/tick)** échoue au budget de
  100 ms en mode `delta` sur **les 9 exécutions** (1 en Campagne 1 + 8 en
  Campagne 2) : 14,4 Mo/tick d'écritures synchrones donnent un **p99 tick
  complet de 113,41–482,78 ms** (commit `sync_data` p99 jusqu'à
  **283,10–400,12 ms**, max **689,37 ms**) et projettent 144 Mo/s avant
  réplication.
- **L'organisation mémoire de l'état domine le coût CPU en charge dense, tandis
  que la cible de ratio de rejeu `< 0,5` n'est pas atteinte :** passer de
  `BTreeMap` + overlay à un stockage contigu par partition avec undo en place
  ramène B en mode `root` de **147,07 ms à 78,95 ms p99** (66,13 ms médiane)
  sur un thread, mais **aucune variante n'atteint le ratio `< 0,5` visé sur B**
  (meilleur : **0,55** en `delta` / **0,62** en `root` avec
  `partitioned/acc-cached`).
- **Le rehash complet par partition casse les charges de commandes dispersées**
  dès lors que des racines par résolution sont exigées (A/C passent de
  **6–10 ms à 115–169 ms** médiane ; même B est plus lent à 51 ms contre 29 ms
  avec empreintes par enregistrement mises en cache).
- **Les checkpoints complets synchrones (144 Mo)** figent le tick 100 pendant
  **237–435 ms** sur la Campagne 2 (pointe à **1 485 ms** ; Campagne 1 :
  **247–326 ms**), prouvant que l'I/O de checkpoint doit sortir du chemin
  critique du tick à grande échelle.

### Décision

1. **« Narrow waist » d'état logique (zéro ontologie applicative) :**
   l'état du monde est modélisé comme des domaines propriétaires versionnés
   adossés à des tables ordonnées partitionnées
   `(DomainId, TableId, PartitionKey, RecordKey) -> OpaqueBytes` plus
   `ObligationRegistry`. Aucun archétype ECS ni index secondaire géré par le
   noyau n'est ajouté au cœur. Un tag `TableKind` extensible réserve la place
   pour des tables colonnaires additives plus tard sans casser la v1.
2. **Chemin de mutation unique pour l'exécution en direct et la reprise :**
   toutes les mutations canoniques passent par un unique chemin déterministe
   `apply(WriteSet)` (jamais deux implémentations divergentes comme `sim.rs` et
   `replay.rs` du prototype, et jamais de clonage complet du monde pour
   annuler).
3. **Geler les racines dans le contrat ; laisser le mode de preuve et
   l'algorithme de hash dans le profil d'exécution :**
   `Resolution` lie `pre_state_root`, `post_state_root` et `events_root`.
   - **Mode de preuve** choisi par le profil d'exécution versionné : la
     matérialisation explicite de `WriteSet` (`delta`) est la **valeur par
     défaut en v1** (préservant la sûreté de reprise D-014 tant que les plugins
     au démarrage sont en Rust natif non isolé et couvrant les charges A/C avec
     une marge de 4,6×–11,6×) ; un mode de preuve compact vérifié par tick
     (`root`) reste une option de profil future explicite pour les charges
     denses une fois les exigences d'isolation/déterminisme remplies.
   - **Algorithme de hash** paramétré dans le profil versionné, jamais figé sur
     une unique primitive cryptographique dans le contrat racine. Tant que des
     racines par résolution sont actives, le hash doit se mettre à jour de
     façon incrémentale par enregistrement modifié plutôt que de rehacher des
     partitions entières.
   - `MigrationActivate` commite un **checkpoint frontière obligatoire** afin
     qu'une reprise ordinaire ne traverse jamais deux profils d'exécution.
4. **Une seule couture publique gelée (discipline Linux / Cordis) :**
   la v1 ne gèle **que le contrat public plugin, commandes et observation**
   ([CONTRACT.md](CONTRACT.md) et [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md)).
   Les coutures d'infrastructure internes — `StateStore`, `AuthorityLog` et
   `Executor` — restent des traits internes propres testés avec des doubles en
   mémoire et à injection de pannes (en remplacement des neuf contrats publics
   de fournisseurs de D-014 sous `K-05`). Aucune couture interne n'est promue
   en ABI publique gelée sans un second fournisseur réel en production et un
   consommateur réel.
5. **Contrats purs prêts pour le parallélisme, v1 séquentielle batchée + DST :**
   - Remplacer `IRng` global mutable et les décisions flottantes par un
     `EntropyScope` entier dérivé de coordonnées (algorithme choisi dans le
     profil d'exécution). Éliminer les compteurs d'ID globaux mutables et
     l'accumulation concurrente d'événements.
   - Les points d'entrée plugins sont purs, sans état et orientés batch ou
     partition.
   - Toute passe de tick autonome observant un instantané immuable $T-1$
     (sémantique BSP) doit déclarer explicitement cette sémantique de lecture
     dès la v1 même en exécution séquentielle, afin que la future exécution
     multi-cœur préserve l'exacte parité binaire.
   - La v1 livre un **exécuteur séquentiel batché** (1 group commit par tick)
     associé à du **Deterministic Simulation Testing (DST) mono-thread** pour
     prouver le consensus distribué, la reprise après crash, les migrations et
     les pannes de stockage. L'exécution multi-cœur (`Block-STM`, `BSP`
     parallèle), les checkpoints CoW asynchrones hors chemin critique,
     `ColumnarTable`, `BoundedDelta` et l'isolation WASM restent des travaux
     additifs de Gate 1-D.
6. **Autorité CP par monde + fédération asynchrone inter-mondes :**
   un monde reste un unique groupe d'autorité CP (1 nœud ou 3 nœuds
   *shared-nothing*, quorum 2). Les partitions locales d'un monde partagent son
   groupe d'autorité et son horloge. La décentralisation pair-à-pair et
   l'autonomie locale opèrent **entre mondes autonomes** via des enveloppes
   asynchrones et des obligations durables, évitant l'effondrement de
   disponibilité d'un couplage *lockstep* multi-groupes ($p^N$).
7. **Fermer la faille de vivacité `CancelCommand` / `CloseTick` :**
   - Le noyau réserve un **budget de plan de contrôle** borné et dédié pour les
     opérations autorisées `CancelCommand` et de contrôle de migration, isolé
     des quotas applicatifs afin que la saturation applicative ne puisse jamais
     renvoyer `LimitExceeded` à une annulation administrative.
   - `CloseTick(T)` ne peut pas clore le tick `T` tant qu'une commande admise
     `Pending` ciblant `T` reste non résolue ; celle-ci atteint soit une
     résolution terminale normale, soit `Cancelled` via la voie de contrôle
     réservée.
8. **Remise à zéro propre du cœur et phases de codage ordonnées :**
   le prototype spatial historique (`Entity`/`Zone`, snapshots JSON, double
   chemin `sim.rs`/`replay.rs`, fixtures `server_d`/`sy_cli` et crates
   *placeholders* inertes) est retiré plutôt que modifié incrémentalement. Le
   workspace repart sur un squelette minimal en 4 crates (`sy_types`, `sy_api`,
   `sy_core`, `sy_infra`) préservant la porte de pureté, Rust `1.95.0` épinglé,
   les contrôles de licences/advisories et les règles CI de DAG de dépendances,
   avec une Rustdoc interne alignée sur les phases de codage ordonnées
   (`Phase 0` à `Phase 4`) dans [ROADMAP.md](ROADMAP.md).

### Conséquences

- La documentation est consolidée en un jeu unique sans redondance (avec un
  miroir français non normatif dans `doc/fr/` et les preuves empiriques
  conservées dans [BENCHMARKS.md](BENCHMARKS.md)) ; `CONTRACT.md` passe en
  `0.4.0-draft` et `PLUGIN_CONTRACT.md` en `0.3.0-draft` (tous deux non gelés).
- **Bases archivées :** le texte intégral des décisions pré-D-015 et les
  critères d'acceptation détaillés de Gate 1-K (`seej/docs/phase1/EXIT_CHECKLIST.md`
  — y compris les frontières de crash `K-08`, les seuils finis de scellement de
  génération, les chemins de migration depuis chaque version stable et les
  messages inter-nœuds bornés, authentifiés et protégés contre le rejeu)
  restent archivés au tag d'archive `archive/pre-d015-docs` (`9662a60`) comme
  référence d'acceptation jusqu'à leur re-spécification et fermeture par tests
  phase par phase. Le dernier prototype local exécutable (avec ses correctifs
  WAL *fail-closed*, RNG v4 et rollback `TickN`) est archivé au tag
  `archive/prototype-v1-final` (`5841a00`), et les deux campagnes labo aux tags
  `archive/lab-campaign-1` (`1c98282`) et `archive/lab-campaign-2` (`5856396`).
- `K-05` est recentré des neuf remplacements de fournisseurs publics vers la
  conformité des coutures internes et les doubles à injection de pannes
  (`StateStore`, `AuthorityLog`, `Executor`) ; `P2-01` est retiré avec la
  suppression du *placeholder* inerte `sy_loader`.
- Le choix du moteur de stockage (`redb`, `Fjall` ou store paginé maison) et la
  construction de hash de production restent des choix internes de profil à
  mesurer sur des charges de stockage réelles au démarrage de la **Phase 1.4**.
- D-015 ne ferme aucun item d'implémentation de Gate 1-K à elle seule ; les
  preuves exécutables dans [ROADMAP.md](ROADMAP.md) restent obligatoires avant
  le gel v1.

---

## Règles d'amendement

- Toute décision structurelle doit être ajoutée ici avec son contexte, ses
  preuves, la décision, ses conséquences et ses liens explicites de
  remplacement.
- Les décisions existantes ne sont jamais réécrites silencieusement ; les
  amendements ajoutent une nouvelle entrée et mettent à jour le tableau de
  statut.
