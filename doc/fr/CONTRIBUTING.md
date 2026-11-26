# Guide de contribution

> *Miroir français de [`CONTRIBUTING.md`](../../CONTRIBUTING.md). La version
> anglaise fait foi en cas d'écart.*

Seej est une infrastructure open-source pour des mondes déterministes et
persistants. Chaque contribution est évaluée au regard du journal des décisions,
des contrats publics, des jalons de sortie et du comportement réel du dépôt.

Commencer par :

- [README.md](README.md) — vue d'ensemble et démarrage rapide ;
- [DECISIONS.md](DECISIONS.md) — décisions acceptées (`D-001`–`D-015`) ;
- [ARCHITECTURE.md](ARCHITECTURE.md) — frontières système et règles de dépendance ;
- [CONTRACT.md](CONTRACT.md) et [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md) — brouillons des contrats v1 ;
- [ROADMAP.md](ROADMAP.md) — jalons et checklist de sortie faisant foi ;
- [SECURITY.md](SECURITY.md) et [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md).

Signaler toute vulnérabilité de sécurité en privé selon
[SECURITY.md](SECURITY.md#signalement-de-vulnérabilité), jamais dans une issue
ou une pull request publique.

## Discipline d'ingénierie

1. **Mesurer avant de complexifier.** Ne pas ajouter d'abstractions spéculatives,
   d'ordonnanceurs parallèles, de moteurs de stockage supplémentaires ni
   d'interfaces internes gelées sans benchmark exécutable ou consommateur réel.
2. **Préserver le chemin de mutation unique.** Les changements canoniques
   entrent par commandes admises et s'appliquent par un seul chemin déterministe
   de *write-set*.
3. **Ne jamais confondre conception et implémentation.** Ne jamais cocher un
   écart de roadmap parce qu'un texte existe ; fermer uniquement avec une preuve
   exécutable commitée.
4. **Garder le cœur minimal.** Les ontologies applicatives (espace, entités,
   économie, IA, narration, rendu, netcode ingame) vivent dans les plugins ou
   les consommateurs externes, jamais dans `sy_core`.

## Changements qui seront refusés

Une contribution est incompatible si elle :

- place des graphismes, du netcode joueur ou une ontologie applicative dans `sy_core` ;
- rompt la règle zéro dépendance externe de `sy_core` ou sa porte de pureté ;
- permet à un client, observateur, pair découvert ou plugin de muter l'état hors
  du chemin canonique commandes / *write-set* ;
- rapporte un succès canonique avant la confirmation durable requise, ou traite
  un ajout WAL local comme une confirmation distribuée ;
- utilise l'horloge murale, l'ordre d'arrivée réseau, l'ordonnancement de threads
  ou un hasard ambiant comme entrée de transition implicite ;
- ignore silencieusement des données opaques, des obligations actives ou des
  variantes obligatoires inconnues ;
- répare silencieusement une corruption ambiguë ou effectue une migration implicite ;
- gèle prématurément une couture d'infrastructure interne sans second
  fournisseur et consommateur réels ;
- présente un comportement cible comme implémenté.

## Validation

Exécuter depuis `seej/server/` avec la chaîne Rust épinglée :

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
```

Pour un changement purement documentaire, vérifier tous les liens locaux,
ancres, commandes et libellés *implémenté* vs *cible*.

## Flux de travail et gouvernance

- Utiliser des branches ciblées (`fix/...`, `feat/...`, `docs/...`,
  `refactor/...`) et des commits atomiques.
- Tout changement architectural ou contractuel significatif exige une décision
  dans [DECISIONS.md](DECISIONS.md) expliquant le problème, les preuves et
  l'impact sur les décisions antérieures.
- Les mainteneurs protègent la cohérence long terme et peuvent refuser une PR
  techniquement correcte qui contredit les décisions acceptées ou ajoute une
  complexité prématurée.
- Toute participation suit le [Code de conduite](../CODE_OF_CONDUCT.md).
