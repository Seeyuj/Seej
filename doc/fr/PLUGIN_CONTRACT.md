# Contrat public plugin de Seej

> **Traduction non normative** de [`seej/docs/PLUGIN_CONTRACT.md`](../../seej/docs/PLUGIN_CONTRACT.md).
> La version anglaise fait foi en cas d'écart.

```text
contract_version: 0.3.0-draft
status:           BROUILLON — NON GELÉ (écart K-01)
```

Ce document définit l'**unique frontière publique visée pour le gel en v1**
(actuellement `BROUILLON — NON GELÉ` jusqu'à `K-01`) : le contrat entre le
noyau de monde Seej et les plugins applicatifs. Il doit rester utilisable par
des auteurs tiers sans lire `sy_core` ni le code interne de stockage ou de
consensus.

---

## 1. Une seule couture publique visée pour le gel

Selon [D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée) :

- **Frontière publique visée pour le gel v1 (`K-01`) :** le **contrat plugin
  applicatif** défini ici (ainsi que les schémas publics de commandes et
  d'observation dans [CONTRACT.md](CONTRACT.md)).
- **Coutures internes cibles en v1 (non gelées publiquement) :** `StateStore`,
  `AuthorityLog` et `Executor`. Ces composants sont conçus comme des traits
  Rust internes destinés à être vérifiés avec des doubles en mémoire et à
  injection de pannes (`K-05`), libres d'évoluer sans casser les plugins.
- **Modèle de confiance et chargement :** la v1 ne charge au démarrage du
  processus que des plugins natifs de confiance approuvés par l'opérateur. Le
  chargement à chaud, le remplacement à chaud et l'exécution isolée (ex. WASM
  avec comptage de fuel déterministe et canonisation des NaN) sont des
  capacités de Gate 1-D conçues pour préserver exactement ce même contrat
  logique.

---

## 2. Manifeste et compatibilité

Chaque plugin déclare un manifeste immuable spécifiant :

- des identifiants stables de composant et d'éditeur ;
- la version sémantique et l'empreinte cryptographique de l'artefact ;
- la version cible du contrat noyau/plugin ;
- les domaines possédés, déclarations de tables (`TableKind`, versions de
  format), types de commandes publics et vues lisibles déclarées ;
- les dépendances, permissions et limites de ressources ;
- les exigences de profil d'exécution déterministe (dont l'algorithme
  d'entropie) ;
- les étapes de migration de données depuis les versions stables antérieures ;
- les types d'obligations de cycle de vie bloquant un retrait silencieux.

L'activation au démarrage vérifie chaque manifeste, empreinte, permission,
dépendance et version de données avant d'autoriser les mutations du monde. Tout
plugin requis manquant, incohérent ou incompatible échoue fermé et bloque
l'activation, la migration ou la re-simulation.

---

## 3. Données de domaine et propriété

Chaque domaine applicatif mutable a **un unique propriétaire logique déclaré**.
Les autres plugins peuvent lire les vues autorisées et ne peuvent muter les
données d'un autre propriétaire **qu'**en invoquant les commandes publiques
possédées par ce domaine à l'intérieur d'une transaction noyau.

### Primitives logiques de données (`K-06`)

Le noyau n'impose aucun modèle obligatoire d'espace, entité, zone, objet,
créature, économie ou joueur. Les domaines stockent leur état canonique
exclusivement dans les conteneurs du noyau — **zéro état canonique caché** ne
peut résider dans la mémoire tas (*heap*) d'un plugin :

1. **`OrderedTable` (base v1) :**
   `(DomainId, TableId, PartitionKey: Bytes, RecordKey: Bytes) -> OpaqueBytes`
   trié lexicographiquement.
   - Les domaines à blob opaque unique sont pris en charge trivialement
     (1 table, 1 partition, 1 clé).
   - Les domaines fins utilisent des clés d'entités/comptes ; les simulations
     denses regroupent les enregistrements dans des compartiments
     `PartitionKey` contigus pour la localité cache et la future exécution
     multi-cœur.
   - Les index secondaires canoniques sont des `OrderedTable` ordinaires
     possédées par le plugin et mises à jour dans le même `WriteSet` atomique.
     Les index d'observation uniquement restent hors de l'état canonique.
   - `TableKind` est extensible afin d'autoriser l'ajout de `ColumnarTable`
     (colonnes scalaires fixes par blocs bornés) en Gate 1-D sans casser les
     schémas v1.
2. **`ObligationRegistry` (base v1) :**
   enveloppes d'obligations visibles du noyau `(obligation_id, owner_domain,
   counterparty_domain, status, due_tick, opaque_payload)` préservées même
   lorsque la charge utile est opaque.

---

## 4. Exécution pure en batch et transactions

Pour préserver le déterminisme, éviter le surcoût d'appel par entité et rester
compatible avec le futur sandboxing WASM :

- **Points d'entrée purs en batch plat :** les plugins ne sont jamais invoqués
  via des callbacks par entité (`on_tick(entity)`) ni par accès direct
  `&mut World`. Ils reçoivent une `StateView` immuable plus un lot de commandes
  ou une unité de travail de partition et renvoient des sorties déterministes :
  ```text
  (StateView, InputBatch, EntropyScope, Context)
      -> Decision + WriteSet + EventBatch + OutboxIntents
  ```
- **Aucun I/O ni thread OS :** les plugins n'effectuent jamais d'I/O
  disque/réseau, ne lisent pas l'horloge murale, n'utilisent pas de hasard
  ambiant et ne créent pas de threads OS.
- **Transactions inter-propriétaires :**
  1. Se lier à un `base_cursor` confirmé et ordonner canoniquement les
     sous-commandes propriétaires.
  2. Chaque propriétaire valide contre l'overlay candidat produit par les
     sous-commandes précédentes et ne propose d'écritures que dans son propre
     domaine.
  3. Les validateurs inter-domaines inspectent le candidat complet (en
     privilégiant des contrôles orientés deltas plutôt que des parcours complets
     de domaine afin que les futurs ordonnanceurs parallèles évitent toute
     sérialisation inutile).
  4. Toutes les écritures et événements des propriétaires se confirment
     atomiquement ou aucun ne s'applique. Un curseur de base périmé fait jeter
     le candidat et déclenche un recalcul complet.
- **Indices d'accès optionnels :** les commandes peuvent déclarer des indices
  optionnels de partition/domaine (`AccessHint`) pour le routage et le futur
  ordonnancement parallèle, sans imposer de listes de clés statiques rigides aux
  charges dynamiques.
- **Sémantique d'échec :** seule une évaluation contractuelle déterministe
  produit `Refused`. Une panique plugin, un timeout ou un artefact manquant
  laisse la commande `Pending` (récupérable après correctif ou `CancelCommand`
  autorisé) et ne fabrique jamais de refus métier ni d'effet partiel.

---

## 5. Entropie et sémantique de lecture

- **`EntropyScope` dérivé :** les plugins reçoivent un `EntropyScope` lié au
  contexte fournissant des tirages déterministes exclusivement entiers
  (`fork(stable_key)`, `next_u64`, intervalles bornés, ratios entiers — aucune
  décision `f32`/`f64`). Le *fork* par clé sémantique stable isole les plugins
  entre eux et garde les tirages invariants entre 1 et $N$ cœurs.
- **Sémantique explicite $T-1$ vs séquentielle (`D-015`) :**
  - Les gestionnaires de commandes/transactions observent l'état séquentiel
    exact dans l'ordre canonique `(target_tick, admission_seq)`.
  - Les passes de tick autonomes qui lisent un instantané immuable $T-1$ et
    émettent des intentions réduites dans l'ordre canonique (style BSP)
    **doivent déclarer explicitement cette sémantique d'instantané $T-1$** même
    sur l'exécuteur séquentiel v1. Cela garantit `root(1 cœur) == root(N cœurs)`
    lorsque l'exécution parallèle sera activée plus tard.
- **IDs dérivés de la cause & événements canoniques :** les plugins ne doivent
  pas dépendre de compteurs globaux mutables partagés entre partitions
  indépendantes ; les identifiants générés et les événements tirent leur ordre
  déterministe de leur cause canonique.

---

## 6. Obligations et cycle de vie

- **Migrations :** les plugins possèdent les migrations de leurs tables et
  doivent fournir des étapes de conversion déterministes entre versions. Des
  étapes manquantes bloquent la migration du monde (`K-09`).
- **Retrait / remplacement :** un plugin ayant des données possédées ou des
  obligations actives ne peut être détaché par simple suppression de son
  binaire. Le retrait exige une migration explicite des données ainsi que soit
  le transfert confirmé de chaque obligation active vers un successeur
  compatible activé, soit sa résolution confirmée selon les lois du monde.
  L'archivage ne peut détacher le code que si les obligations sont terminales
  ou si le monde entier est archivé et ne peut plus avancer.
- **Artefacts historiques :** les opérateurs archivent les binaires de plugins
  nécessaires à la re-simulation historique ; le noyau consigne les empreintes
  exactes et refuse la re-simulation si des artefacts référencés sont
  indisponibles.

---

## 7. Extensions communautaires optionnelles

Moteurs spatiaux, économies, agents/IA, moteurs de rendu, netcode ingame et
systèmes multirésolution sont des plugins communautaires optionnels, jamais des
livrables du noyau.

### Multirésolution / résolution causale (motif optionnel)

Un plugin communautaire de multirésolution peut représenter des régions ou
groupes inactifs à une granularité plus grossière et matérialiser les détails
plus tard **uniquement** via des commandes canoniques admises sous une
politique d'approximation déclarée par le monde :

- Les faits passés établis, identités protégées, lois et obligations actives
  restent exacts et ne peuvent être approximés.
- Les résultats futurs ne peuvent diverger d'une exécution hypothétique
  entièrement détaillée que là où la politique déclarée l'autorise
  explicitement ; le déterminisme signifie qu'à entrées et politique égales,
  les transitions sont égales.
- **Neutralité d'observation :** le mouvement de caméra, les abonnements ou les
  requêtes d'observateurs ne déclenchent jamais de matérialisation canonique ni
  ne modifient la fidélité de simulation. Les changements de fidélité n'entrent
  que par commandes canoniques validées.

---

## 8. Preuves de conformité v1

Avant que `K-01` ne gèle ce contrat, la suite de tests doit prouver :

1. **Véritable extension d'écriture externe (`K-04`) :** maintenue hors de
   `sy_core`, n'important que les contrats publics, déclarant des données
   `OrderedTable` non spatiales, des commandes et des validateurs, et survivant
   à la reprise après crash.
2. **Fixtures de transactions & obligations (`K-06`) :**
   - avec une loi de solde non négatif, deux retraits ordonnés de 7 sur un
     solde de 10 ne peuvent réussir tous les deux ;
   - paiement + transfert d'objet entre deux propriétaires se confirment
     atomiquement ou pas du tout ;
   - un candidat sur base périmée est jeté et recalculé intégralement ;
   - une panique/défaillance d'un participant ne produit aucun effet partiel ni
     aucun `Refused` fabriqué ;
   - une obligation active à échéance future bloque le retrait de son
     propriétaire jusqu'à transfert confirmé ou résolution ;
   - une passe autonome $T-1$ déclarée produit des racines identiques quel que
     soit l'ordre d'évaluation des partitions.
