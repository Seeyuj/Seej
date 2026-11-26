# AGENTS.md — Seej (miroir FR)

> *Miroir français de [`AGENTS.md`](../../AGENTS.md). La version anglaise fait
> foi en cas d'écart.*

Seej est une infrastructure open-source pour des mondes bac à sable
déterministes et persistants. [D-012](DECISIONS.md#d-012--noyau-de-monde-distribué-et-extensible)
a posé la direction de noyau distribué extensible,
[D-013](DECISIONS.md#d-013--sémantique-opérationnelle-des-pannes) et
[D-014](DECISIONS.md#d-014--sémantique-des-frontières-de-la-première-version-stable)
ont fixé la sémantique opérationnelle des pannes, et
[D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée) ancre le cœur
minimal, l'unique couture plugin gelée et la feuille de route incrémentale sur
des mesures réelles.

Arbitrer les compromis dans cet ordre : durabilité architecturale, exactitude,
déterminisme, persistance et reprise, maintenabilité, testabilité,
observabilité, performance, puis vitesse d'implémentation.

## Invariants non négociables

- **Autorité CP par monde :** un monde possède une seule histoire confirmée,
  détenue par son groupe de nœuds autorisés (1 nœud ou 3 nœuds *shared-nothing*,
  quorum 2). La découverte réseau n'accorde jamais l'autorité et une partition
  minoritaire ne peut pas écrire. Des termes d'autorité durables clôturent les
  coordinateurs ; des versions de configuration confirmées gouvernent le
  membership. La décentralisation pair-à-pair opère entre mondes autonomes,
  jamais via un sharding synchrone verrouillé d'un même monde.
- **Fonctionnement headless :** un monde évolue sans interface, rendu, client,
  agent ni netcode ingame. Un déploiement mono-nœud et un déploiement à trois
  nœuds obéissent au même contrat public.
- **Déterminisme :** la même genèse ou le même checkpoint, les mêmes entrées
  admises ordonnées, le même calendrier de ticks et le même profil d'exécution
  produisent exactement les mêmes transitions. `sy_core` ne peut dépendre ni de
  l'horloge murale, ni du timing réseau, ni de l'ordre du système de fichiers,
  ni de l'ordonnancement de threads, ni d'un hasard ambiant, ni d'une itération
  non déterministe. L'entropie est dérivée de coordonnées canoniques
  (`EntropyScope`), jamais d'un RNG global mutable ni de décisions flottantes.
- **Chemin de mutation unique et durabilité :** toute mutation canonique — en
  direct comme en reprise — passe par un seul chemin déterministe
  `apply(write_set)` sur des tables ordonnées partitionnées. Ne jamais rapporter
  de succès canonique ni publier d'effets avant la confirmation durable requise.
  Échouer fermé (*fail closed*) sur erreur de persistance ; suspendre les
  mutations sur perte de quorum tout en permettant une inspection en lecture
  seule explicitement marquée périmée (*stale*). `CancelCommand` dispose d'un
  budget de contrôle réservé afin que des quotas applicatifs saturés ne puissent
  jamais bloquer le déblocage d'une commande `Pending`.
- **Une seule couture publique gelée, pas de sur-ingénierie :** la v1 ne gèle
  que le contrat public plugin/commandes/observation (`CONTRACT.md` +
  `PLUGIN_CONTRACT.md`). Les coutures d'infrastructure internes (`StateStore`,
  `AuthorityLog`, `Executor`) restent des traits internes testables — ne jamais
  geler une couture interne sans second fournisseur réel et consommateur réel,
  et ne jamais ajouter d'optimisations complexes (`Block-STM`, `ColumnarTable`,
  hashes cryptographiques sur mesure, WASM) avant un besoin mesuré.
- **Compatibilité :** toute modification affectant le rejeu exige une version de
  contrat ou de profil explicite. Les données stables du noyau conservent un
  chemin de migration documenté ; les plugins possèdent leurs migrations.
  `MigrationActivate` commite un checkpoint frontière obligatoire afin qu'une
  reprise ordinaire ne traverse jamais deux profils d'exécution.
- **Architecture :** `sy_core` dépend uniquement de `sy_types` et `sy_api` (zéro
  crate externe) et doit passer la porte de pureté. L'I/O réel reste hors du
  moteur pur.

## Statut actuel

`seej/server/` contient aujourd'hui un prototype local mono-processus
(`server_d`, `sy_cli`) avec un WAL v1 *fail-closed*, des snapshots v3/v4 et des
fixtures spatiales `Entity`/`Zone`. Le traiter comme une base jetable : pour
construire le cœur générique D-015, repartir sur du code neuf et propre est
explicitement préféré au maintien des types spatiaux ou des snapshots JSON du
prototype.

Ne jamais présenter les capacités cibles (domaines génériques, consensus,
observation réseau, migrations, isolation WASM, chargement à chaud) comme
implémentées.

## Contexte à la demande

| Sujet | Référence FR |
| --- | --- |
| Décisions architecturales (`D-001`–`D-015`) | [DECISIONS.md](DECISIONS.md) |
| Architecture, modèle d'état, coutures, pureté | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Contrat public noyau, autorité et commandes | [CONTRACT.md](CONTRACT.md) |
| Frontière publique plugin gelée v1 | [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md) |
| Jalons et checklist de sortie | [ROADMAP.md](ROADMAP.md) |
| Prototype actuel et benchmarks | [PROTOTYPE.md](PROTOTYPE.md) |
| Modèle de menace et sécurité | [SECURITY.md](SECURITY.md) |

## Vérification

Exécuter les vérifications Rust depuis `seej/server/` avec la chaîne épinglée
(`rust-toolchain.toml`) :

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
```
