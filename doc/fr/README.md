# Seej

> *Miroir français de [`README.md`](../../README.md). La documentation anglaise
> fait foi en cas d'écart.*

> Noyau de mondes déterministes et persistants avec un cœur minimal et une
> frontière plugin gelée.

Seej est une infrastructure open-source pour des mondes bac à sable persistants
qui évoluent sans joueur, sans moteur de rendu et sans coordinateur central
global. Un monde logique tourne sur un nœud autorisé ou sur un petit groupe
d'autorité (trois nœuds, quorum deux) autour d'une unique histoire confirmée.
Les modèles de monde, systèmes spatiaux, économies, agents, politiques de
multirésolution, moteurs de rendu et netcode ingame restent hors du noyau sous
forme de plugins et de consommateurs.

- Dépôt : [https://github.com/Seeyuj/Seej](https://github.com/Seeyuj/Seej)
- Actualités : [https://x.com/Seeyuj](https://x.com/Seeyuj)

## Philosophie de conception

Seej réunit trois principes d'ingénierie systèmes — réconciliés avec le
déterminisme et la cohérence distribuée plutôt que traités comme des slogans :

- **Petit cœur, une seule couture gelée (Unix / Linux) :** garder un noyau petit
  et cohérent ; geler uniquement le **contrat public plugin et externe** (comme
  l'ABI des appels système Linux) tout en gardant les coutures d'infrastructure
  internes (`StateStore`, `AuthorityLog`, `Executor`) propres, testées et
  évolutives.
- **Capacités composables sur un substrat de mutation unique (`dsh` / Cordis) :**
  les fonctionnalités applicatives sont des plugins, pas des patchs du cœur,
  composés autour d'une unique primitive de mutation — des *write-sets*
  ordonnés et journalisés appliqués par un seul chemin déterministe.
- **Mondes autonomes, nœuds sans ressources partagées (*shared-nothing*) :**
  aucun registre global ni SPOF. Chaque monde maintient une cohérence forte (CP)
  au sein de son propre groupe d'autorité *shared-nothing* ; la décentralisation
  et la coopération pair-à-pair opèrent entre mondes autonomes par enveloppes
  asynchrones.

## Statut

La documentation utilise trois libellés :

- **Implémenté :** présent dans `seej/server/` et couvert par des tests nommés.
- **Cible :** obligatoire avant le gel du contrat v1 (suivi dans
  [ROADMAP.md](ROADMAP.md)).
- **Plus tard :** durcissement tiré par l'usage ou écosystème communautaire.

**Aujourd'hui (Phase 0 terminée — socle vierge) :** selon
[D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée), le
prototype spatial historique (`Entity`/`Zone`, snapshots JSON, double chemin
`sim.rs`/`replay.rs`) a été retiré. `seej/server/` contient un squelette
workspace Rust minimal et propre en 4 crates (`sy_types`, `sy_api`, `sy_core`,
`sy_infra`) avec `#![deny(missing_docs)]`, sa Rustdoc de phases intégrée, la
porte de pureté `sy_core`, Rust `1.95.0` épinglé et tous les garde-fous CI —
prêt à démarrer la **Phase 1.1** dans [ROADMAP.md](ROADMAP.md). Aucun démon,
stockage, consensus ou hôte de plugins n'est encore implémenté.

## Démarrage rapide (vérifier le workspace et générer la Rustdoc)

Nécessite la chaîne Rust épinglée dans `seej/server/rust-toolchain.toml` :

```bash
cd seej/server

python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --open
```

## Carte de la documentation (miroir FR)

Chaque document porte une responsabilité unique, sans spécification dupliquée :

| Document (FR) | Source EN | Rôle |
| --- | --- | --- |
| [ARCHITECTURE.md](ARCHITECTURE.md) | [doc/ARCHITECTURE.md](../ARCHITECTURE.md) | Modèle système, état logique minimal, coutures, règles de dépendance et arborescence |
| [DECISIONS.md](DECISIONS.md) | [doc/DECISIONS.md](../DECISIONS.md) | Journal des décisions architecturales (`D-001` à `D-015`) et filiation |
| [ROADMAP.md](ROADMAP.md) | [doc/ROADMAP.md](../ROADMAP.md) | Plan de codage ordonné (`Phase 0`–`Phase 4`) et checklist de sortie faisant foi |
| [CONTRACT.md](CONTRACT.md) | [seej/docs/CONTRACT.md](../../seej/docs/CONTRACT.md) | Brouillon du contrat noyau, commandes, autorité, reprise, migration et observation |
| [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md) | [seej/docs/PLUGIN_CONTRACT.md](../../seej/docs/PLUGIN_CONTRACT.md) | Brouillon de l'unique frontière publique gelée en v1 (plugins, domaines, transactions) |
| [BENCHMARKS.md](BENCHMARKS.md) | [seej/docs/BENCHMARKS.md](../../seej/docs/BENCHMARKS.md) | Mesures empiriques de laboratoire fondant D-015 (stockage, hachage, modes de preuve) |
| [SECURITY.md](SECURITY.md) | [doc/SECURITY.md](../SECURITY.md) | Modèle de menace, frontières de confiance, posture opérationnelle et signalement |
| [AGENTS.md](AGENTS.md) | [AGENTS.md](../../AGENTS.md) | Invariants non négociables, règles anti-sur-ingénierie et vérifications |
| [CONTRIBUTING.md](CONTRIBUTING.md) | [CONTRIBUTING.md](../../CONTRIBUTING.md) | Règles de contribution, vérifications et gouvernance |

## Licence

Seej est distribué sous [licence MIT](../../LICENSE).
