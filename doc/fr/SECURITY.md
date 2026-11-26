# Politique de sécurité

> *Miroir français de [`doc/SECURITY.md`](../SECURITY.md). La version anglaise
> fait foi en cas d'écart.*

La sécurité est une propriété structurelle des contrats d'autorité, de
persistance, de plugins et de compatibilité de Seej. Un monde corrompu en
silence est pire qu'un monde explicitement indisponible.

## Périmètre

Dans le périmètre : le code noyau Seej maintenu, les adaptateurs stockage et
consensus, les outils CLI/opérateurs, les frontières contractuelles publiques et
les contrôles build/supply-chain du dépôt. Une vulnérabilité dans la validation,
les permissions ou les frontières d'isolation reste dans le périmètre même si
elle est déclenchée par un plugin tiers.

Hors périmètre sauf adoption : plugins communautaires, moteurs de rendu,
netcode ingame et infrastructure hôte des opérateurs.

## Posture de sécurité actuelle et cible

- **État actuel du dépôt (`seej/server/` — Phase 0 socle vierge) :** squelette
  workspace Rust en 4 crates (`sy_types`, `sy_api`, `sy_core`, `sy_infra`) avec
  garde-fous actifs de compilation, pureté et chaîne d'approvisionnement
  (`check_sy_core_purity.py`, `cargo-audit`, `cargo-deny`, contrôle du DAG de
  dépendances). Aucun démon, moteur de stockage, consensus, authentification ou
  isolation de plugin n'existe encore.
- **Cible v1 (Gate 1-K) :** groupes d'autorité tolérants aux pannes franches
  (CFT) de 1 ou 3 nœuds authentifiés par les opérateurs (quorum 2). Les nœuds
  autorisés byzantins sont hors modèle de panne v1 ; l'admission non
  authentifiée, le contournement de privilèges, la falsification de messages et
  les écritures d'autorité périmée sont des failles de sécurité.
- **Plugins au démarrage en v1 :** composants natifs de confiance approuvés par
  l'opérateur avec vérification de manifeste et d'empreinte. Ils tournent avec
  les privilèges du processus et ne doivent jamais être présentés comme
  sandboxés.
- **Plus tard (Gate 1-D / Évolution 2) :** exécution isolée des plugins
  applicatifs (ex. WASM avec *fuel metering* déterministe et permissions
  explicites).

## Actifs et invariants

Protéger au minimum :

- une seule histoire confirmée et un `world_id` stable par monde ;
- l'identité des commandes, les slots/incarnations/générations producteurs
  bornés, l'expiration exacte et la non-réutilisation des IDs expirés ;
- l'autorisation, l'idempotence et le **budget de contrôle réservé** des
  opérations de contrôle noyau (`CancelCommand`) afin que l'épuisement des
  quotas applicatifs ne puisse jamais provoquer un déni de service permanent ;
- les checkpoints, l'historique retenu, les obligations actives et les barrières
  de migration ;
- les termes d'autorité, les configurations de membership confirmées et les
  empreintes des composants ;
- les identifiants de transport, clés de signature et diagnostics expurgés (ne
  jamais journaliser de secrets).

## Frontières de confiance

- **Nœuds :** la découverte n'accorde jamais le membership. Les connexions se
  lient à un `node_id` authentifié, au monde, au terme d'autorité, à la version
  de configuration et à un profil d'exécution compatible. Un nœud périmé ou
  minoritaire ne peut ni confirmer de mutations ni prétendre à une lecture
  courante.
- **Producteurs et observateurs :** toute entrée externe est non fiable.
  Authentifier, appliquer les limites de taille/cardinalité et valider avant
  l'exécution pure. Les observateurs ne reçoivent que les domaines autorisés et
  un préfixe confirmé avec fraîcheur explicite ; sous charge, la lecture est
  limitée ou refusée avant d'affecter la simulation canonique.
- **Plugins :** l'installation n'implique aucune permission. Un plugin ne peut
  muter directement les tables d'un autre propriétaire, supprimer des
  obligations actives ou tourner avec des artefacts/empreintes discordants.
- **Stockage :** échouer fermé sur média plein ou lecture seule, erreur d'ajout,
  de synchronisation, de renommage ou d'I/O générique. Séparer récupération de
  queue tronquée sûre, corruption et quarantaine ; ne jamais réparer
  silencieusement une preuve suspecte. Les CRC de stockage et racines
  sémantiques vérifient l'intégrité, jamais l'authentification des pairs.

## Signalement de vulnérabilité

Ne **pas** signaler de vulnérabilité via une issue, discussion ou pull request
publique.

Signaler en privé à : **security@seej.org**

Inclure le commit/version affecté, la frontière touchée, l'impact potentiel sur
l'état du monde, des étapes de reproduction sûres et une piste de remédiation si
disponible.

## Responsabilités de l'opérateur

Les opérateurs sécurisent les hôtes, comptes, clés, terminaux réseau, copies de
sauvegarde, listes de plugins approuvés et archives d'artefacts historiques
nécessaires à la re-simulation. Tant que l'isolation plugin n'existe pas, isoler
les plugins natifs au niveau OS ou conteneur et suivre les procédures de reprise
documentées sans modifier manuellement les fichiers du store canonique.

## Documents liés

- [DECISIONS.md](DECISIONS.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [ROADMAP.md](ROADMAP.md)
- [CONTRACT.md](CONTRACT.md)
- [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md)
- [BENCHMARKS.md](BENCHMARKS.md)
