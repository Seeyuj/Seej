# Contrat noyau et autorité de Seej

> *Miroir français de [`seej/docs/CONTRACT.md`](../../seej/docs/CONTRACT.md).
> La version anglaise fait foi en cas d'écart.*

```text
contract_version: 0.4.0-draft
status:           BROUILLON — NON GELÉ (écart K-01)
```

Ce document est l'unique brouillon normatif du contrat du noyau de monde Seej :
identité, transitions déterministes, mutation d'état, commandes, racines de
résolution, autorité mono-monde, reprise, migration, observation et fédération
inter-mondes. Les contrats côté plugins vivent dans
[PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md). Les mesures empiriques de laboratoire
vivent dans [BENCHMARKS.md](BENCHMARKS.md) ; les phases de codage ordonnées et
l'avancement d'implémentation vivent dans [ROADMAP.md](ROADMAP.md).

Tout énoncé sans mention **Implémenté** décrit la cible v1. Après le gel v1
(`K-01`), tout changement affectant le rejeu exige une incrémentation explicite
de version de contrat ou de profil d'exécution.

---

## 1. Frontière du noyau et identité

Seej est un noyau logique de monde qui possède :

- l'identité stable d'un monde et sa genèse versionnée ;
- le temps simulé canonique (`Tick`) ;
- l'admission de commandes, l'identité durable, la rétention bornée de statut
  et l'annulation par plan de contrôle réservé ;
- l'orchestration des domaines plugins, les transactions inter-propriétaires
  atomiques et les obligations actives ;
- la matérialisation par chemin unique de *write-sets*, les racines d'état
  sémantiques, la persistance, la reprise et la migration coordonnée ;
- la confirmation d'autorité mono-monde, les transitions de membership et
  l'observation confirmée.

Les ontologies applicatives (espace, entités, zones, économies, agents, moteurs
de rendu, netcode ingame) restent hors du noyau.

### Identité & genèse (`P0-06`, `P0-07`)

- `world_id` est une identité stable indépendante de la graine, de
  `genesis_hash`, de l'identité de nœud, du chemin disque et du membership
  d'autorité.
- Un `GenesisSpec` versionné et hachable consigne la politique d'aléa, les
  données initiales de domaines, le profil d'exécution, le profil de limites et
  la configuration d'autorité initiale.
- `genesis_hash` prouve le contenu de la genèse mais ne constitue pas
  l'identité du monde.
- Un réplica conserve `world_id` ; un clone indépendant en alloue un nouveau.
- Le chargement refuse toute incohérence de monde, genèse, profil ou
  métadonnées d'autorité.

---

## 2. Transition déterministe et entropie

Les transitions canoniques sont des fonctions pures de :

```text
état confirmé + entrées admises ordonnées + tick + profil d'exécution actif
```

Le moteur pur ne lit jamais l'horloge murale, le timing d'arrivée des paquets,
l'ordre du système de fichiers, l'ordonnancement des threads, un hasard ambiant
ou les observateurs connectés. Le non-déterminisme externe (LLM, capteurs,
horloges) n'entre que sous forme de commandes bornées, validées et persistées.

- **Compteurs vérifiés (`P0-04`) :** les compteurs de tick, événement, séquence
  de commande/génération, incarnation producteur, expiration, terme d'autorité
  et version de configuration échouent fermé (*fail closed*) en cas de
  débordement — jamais de bouclage ni de saturation silencieuse.
- **Aucun flottant non contraint (`P0-03`) :** les décisions canoniques
  utilisent des représentations entières ou à virgule fixe explicites.
- **Entropie dérivée (`EntropyScope`) :** au lieu d'un unique curseur RNG
  global mutable, l'entropie est dérivée de façon déterministe depuis les
  coordonnées canoniques
  `(world_seed, entropy_epoch, tick, admission_seq | phase, domain_id)` et
  dérivée (*fork*) par clé stable. L'algorithme PRF/PRNG à compteur est déclaré
  dans le profil d'exécution.
- **Profil d'exécution (`P0-05`) :** épingle la version du contrat noyau, le
  mode de preuve, l'algorithme de hash d'état, l'algorithme d'entropie, les
  manifestes et empreintes des plugins actifs, le profil de limites et la
  politique de rétention. Les plateformes supportées annoncent leur profil de
  parité vérifié plutôt que de prétendre à une identité binaire universelle non
  testée.

---

## 3. État du monde et chemin de mutation unique

L'état d'un monde sépare l'**état canonique** (valeurs affectant les
transitions futures ou l'histoire établie) de l'**état opérationnel** (chemins
locaux, sockets, caches, baux, diagnostics).

- **Modèle d'état logique (`K-06`) :** domaines propriétaires versionnés
  adossés à des tables ordonnées partitionnées
  `(DomainId, TableId, PartitionKey, RecordKey) -> OpaqueBytes` plus les
  enveloppes `ObligationRegistry` visibles du noyau.
- **Chemin de mutation unique (`D-015`) :** toute mutation canonique — en
  direct comme en reprise après crash — construit un `WriteSet` déterministe
  sur une `StateView` immuable et le commite via une unique opération canonique
  `apply(WriteSet)`.
- **Atomicité inter-propriétaires (dans un monde) :** une transaction se lie à
  un curseur de base confirmé et ordonne canoniquement ses sous-commandes
  propriétaires. Chaque propriétaire ne mute que son domaine dans un overlay
  candidat privé tout en observant les sous-commandes antérieures ; les
  validateurs inter-domaines inspectent le candidat complet. Les effets se
  confirment atomiquement ou pas du tout. Si la tête confirmée avance avant
  confirmation, le candidat est jeté et recalculé intégralement depuis le vrai
  prédécesseur canonique.
- **Aucun refus fabriqué :** un crash plugin, un timeout, un artefact manquant
  ou une panne de stockage/quorum ne produit jamais une décision `Refused`
  déterministe ; la commande reste `Pending` ou la requête renvoie
  `Unavailable`.

---

## 4. Temps, commandes et plan de contrôle

`Tick` est l'unique horloge canonique d'un monde. Il n'existe aucune horloge
globale entre mondes, et les réplicas n'avancent jamais leurs ticks
indépendamment. La cadence temps réel et les politiques de surcharge/rattrapage
sont des paramètres de configuration qui ne peuvent affaiblir la durabilité, le
quorum ni la neutralité d'observation.

### Enveloppe de commande (`K-08`, `P0-08`)

Un `CommandEnvelope` lie :

- l'identité stable de commande : `(world_id, producer_slot,
  producer_incarnation, admission_generation, producer_seq)` ;
- `target_tick` et `status_window_ticks` fini strictement positif fixé à
  l'admission ;
- `owner_contract` + version, la charge utile canonique et
  `intention_fingerprint` immuable couvrant monde, tick cible, fenêtre de
  statut, contrat, charge utile et contexte d'autorisation affectant la
  validation (hors métadonnées de corrélation transport) ;
- `admission_seq` canonique une fois confirmé.

### Deux états durables

1. **Admission** lie l'enveloppe à `admission_seq` et crée un état durable
   `Pending` sans effet applicatif.
2. **Résolution** consigne une décision terminale — `Succeeded`, `Refused` ou
   `Cancelled` — accompagnée des événements canoniques, des racines et de la
   preuve de transition. Appliquer une résolution confirmée `Succeeded` est
   l'unique façon pour un effet applicatif de devenir canonique. Admission et
   résolution peuvent commiter dans un même lot de consensus à condition que la
   reprise préserve les deux états logiques.

Les retours de requête/consultation `Conflict`, `Expired`, `GenerationSealed`,
`LimitExceeded`, `NotFound`, `Unavailable` et `WorldPaused` sont des résultats
transitoires ou de recherche et ne remplacent jamais un état terminal stocké.

### Clôture de tick, commandes `Pending` et budget de contrôle réservé (`D-015`)

Les commandes s'exécutent par `(target_tick, admission_seq)`. Clore le tick `T`
par un `CloseTick(T)` confirmé rejette les commandes authentiquement nouvelles
ciblant `T` en `Refused` (jamais reciblées silencieusement) ; la déduplication
précède les vérifications de retard.

- **Règle de clôture de tick :** `CloseTick(T)` **ne peut pas** clore le tick
  `T` tant qu'une commande admise `Pending` ciblant `T` reste non résolue. Une
  telle commande atteint soit une résolution terminale normale
  (`Succeeded`/`Refused`), soit `Cancelled` par une opération de contrôle
  autorisée.
- **`CancelCommand` autorisé :** opération de contrôle noyau confirmée portant
  sa propre identité bornée, son demandeur autorisé, sa commande cible et son
  motif. Elle ne résout qu'une commande encore `Pending`, ne produit aucun
  effet applicatif, ne peut effacer un état confirmé ni solder des obligations
  applicatives, et court contre l'exécution normale strictement par l'ordre
  confirmé (le premier enregistrement terminal gagne).
- **Budget réservé du plan de contrôle :** le profil de limites alloue un
  budget de capacité dédié et isolé pour les opérations de contrôle noyau
  (`CancelCommand` et contrôle de migration), séparé des quotas applicatifs.
  Des limites applicatives saturées ne doivent jamais renvoyer `LimitExceeded`
  à des opérations de contrôle autorisées, empêchant tout blocage de vivacité
  lorsqu'une commande `Pending` retient l'avancement des ticks.

### Déduplication, expiration exacte et générations bornées (`K-08`, `K-10`)

Ordre de recherche pour une soumission ou consultation :

1. Vérifier l'incarnation de `producer_slot`. Une incarnation plus ancienne
   renvoie immédiatement `Expired` via la marque haute d'incarnation durable
   par slot.
2. Vérifier le plancher durable de générations fermées de l'incarnation active.
   À ou sous ce plancher renvoie `Expired`.
3. Vérifier que la génération a été ouverte par un `OpenGeneration` confirmé
   (qui scelle atomiquement la génération ouverte précédente). Chercher
   l'identifiant de commande.
4. Pour une résolution terminale au tick canonique `R`, l'arithmétique vérifiée
   fixe `expiration_tick = R + status_window_ticks`. Dès que
   `confirmed_tick >= expiration_tick`, renvoyer `Expired` (même si les
   empreintes diffèrent) ; la compaction peut remplacer le résultat détaillé
   par une *tombstone* de non-réutilisation jusqu'à fermeture de la génération.
5. Pour une entrée existante non expirée : empreinte identique renvoie son
   statut durable (avant les contrôles de tick tardif et de pause migration) ;
   empreinte différente renvoie `Conflict`.
6. Si la pause d'admission pour migration est active, toute nouvelle soumission
   applicative renvoie `WorldPaused` avec identité/phase de migration et ne
   crée aucune Admission.
7. Pour un ID inutilisé : une génération `Open` procède à l'admission (ou
   `NotFound` en consultation de statut) ; une génération `Sealed` renvoie
   `GenerationSealed` en soumission.

Une commande `Pending` n'expire jamais et ne peut être compactée. Des limites
persistées finies bornent slots producteurs, générations non fermées, nombres
et octets de commandes, travaux en attente, résultats et *tombstones* — en
renvoyant `LimitExceeded` sans muter l'état ni prolonger les fenêtres promises.
Une fois toutes les commandes d'une génération `Sealed` terminales et expirées,
`CloseGeneration` confirmé avance le plancher de générations fermées ; retirer
une incarnation exige que toutes ses générations soient fermées, après quoi la
réaffectation du slot incrémente son incarnation monotone.

---

## 5. Résolution, racines et modes de preuve

Un tick confirmé commite un lot group-committé (`TickCommit`) contenant les
admissions ordonnées, opérations de contrôle et résolutions. Chaque
résolution/tick confirmé lie :

- `pre_state_root` et `post_state_root` (racines d'état sémantiques
  indépendantes du layout physique de stockage, `P1-04`, `P1-20`) ;
- `events_root` (engageant le flux canonique d'événements émis pour les
  observateurs) ;
- `execution_profile_hash`, `authority_term`, `config_version` et le curseur
  causal.

### Racines gelées vs mode de preuve et hash dans le profil (`D-015`)

- **Les racines sont gelées dans le contrat v1.** Une incohérence de
  prédécesseur, un trou de curseur ou une divergence de racine post-transition
  échoue fermé immédiatement.
- **L'algorithme de hash d'état vit dans le profil d'exécution**, jamais figé
  sur un unique algorithme permanent dans le contrat racine. Tant que les
  racines par résolution sont vérifiées, le hash se met à jour de façon
  incrémentale par enregistrement modifié (les benchmarks montrent que le
  rehash complet par partition détruit les charges de commandes dispersées).
- **Le mode de preuve vit dans le profil d'exécution :**
  - **`delta` (défaut v1) :** les enregistrements confirmés portent le
    `WriteSet` canonique et les événements ; reprise et *followers*
    matérialisent l'état mécaniquement via `apply(WriteSet)` sans invoquer de
    code plugin applicatif.
  - **`root` (futur profil opt-in) :** les enregistrements confirmés portent
    des entrées ordonnées compactes, les verdicts, `post_state_root` et
    `events_root` sans deltas internes massifs, pour les charges autonomes
    denses une fois le déterminisme plugin garanti par isolation ou profils
    vérifiés.

Les CRC de stockage et les racines d'état sémantiques sont des contrôles
d'intégrité, jamais une authentification de pairs.

---

## 6. Autorité mono-monde et consensus

Chaque monde tourne sur un seul groupe d'autorité autorisé (1 nœud local, ou 3
nœuds *shared-nothing* avec quorum 2 sous pannes franches, pannes de stockage
et partitions réseau). Les nœuds byzantins et l'admission sans permission sont
hors périmètre v1.

- **Deux clôtures monotones :** `authority_term` clôture les coordinateurs lors
  des élections ; `config_version` gouverne les changements de membership.
  Chaque message d'autorité lie les deux.
- **Une seule histoire confirmée :** un ajout WAL local n'est pas un commit
  distribué. Le succès canonique, l'exposition d'état et la publication aux
  observateurs n'ont lieu qu'après confirmation par quorum. La perte de quorum
  suspend les mutations ; les nœuds peuvent servir un état lecture seule
  explicitement marqué périmé (*stale*).
- **Transition de membership de référence (consensus conjoint) :**
  1. Les nouveaux nœuds rejoignent en *learners* non votants, vérifient le
     profil d'exécution et rattrapent leur retard.
  2. La configuration courante confirme une configuration conjointe exigeant
     des majorités à la fois dans l'ancien et dans le nouvel ensemble de
     votants.
  3. La configuration finale se confirme sous la règle conjointe avant que les
     votants retirés ne perdent leur autorité.
  - Un votant configuré hors ligne ou incompatible conserve son rôle et son
    poids de quorum jusqu'à son retrait/rétrogradation explicite par
    reconfiguration conjointe ; il ne peut ni voter ni prétendre à des lectures
    courantes tant qu'il n'est pas compatible et à jour.
  - Terme, vote, membership et log confirmé durables survivent au redémarrage
    complet ; les fichiers de configuration locaux ne surchargent jamais le
    membership confirmé.
- **Nœud de retour :** peut réconcilier un suffixe local explicitement non
  confirmé ; tout conflit dans un préfixe déjà confirmé est mis en quarantaine
  et jamais écrasé.
- **Panne de stockage local :** le nœud touché échoue fermé ou passe en
  quarantaine avant d'accepter du travail non durable ; le groupe continue
  uniquement tant que le quorum survit.

---

## 7. Reprise, migration et rétention

### Trois curseurs distincts

- `confirmed_cursor` — plus haut enregistrement d'histoire canonique confirmé
  par le quorum.
- `checkpoint_cursor` — position confirmée atomiquement liée à l'état, au
  profil d'exécution et à la racine sémantique d'un checkpoint.
- `applied_cursor` — plus haute position confirmée matérialisée dans une
  instance locale d'état (avance atomiquement avec l'état ; une instance en
  retard ne peut servir de lectures courantes).

### Opérations distinctes

- **Reprise :** valider un checkpoint compatible à `checkpoint_cursor`, puis
  appliquer dans l'ordre les enregistrements confirmés conservés après lui (en
  vérifiant les racines prédécesseur et post-transition ; sans jamais
  re-décider les commandes terminales confirmées ni ré-invoquer directement des
  services externes — les consommateurs externes dédupliquent les événements
  identifiés redélivrés).
- **Re-simulation :** opération distincte de diagnostic/audit recalculant les
  commandes conservées avec les artefacts exacts archivés et comparant les
  racines. Refuse de s'exécuter si un artefact référencé manque.
- **Migration :** conversion explicite entre versions stables à une frontière
  confirmée.
- **Clone :** crée un nouveau `world_id` depuis un état exporté.

### Migration coordonnée en 5 phases (`K-09`)

Changements de membership et migration sont sérialisés (la version de
configuration reste figée de la préparation jusqu'à l'activation ou l'abandon ;
les élections peuvent toujours faire avancer `authority_term`).

1. **`MigrationPrepare` :** identifie ancien/nouveau profil d'exécution, étapes
   de conversion, artefacts requis et `config_version` fixe ; met en pause
   toutes les nouvelles admissions applicatives (`WorldPaused`).
2. **Drainage complet :** chaque commande applicative `Pending` de l'ancien
   profil atteint `Succeeded`, `Refused` ou `Cancelled` autorisé (via la voie
   de contrôle réservée).
3. **`MigrationBarrier` :** fixe l'état exact, la racine et le curseur de
   l'ancien profil après drainage complet. Aucun tick de monde ni aucune
   mutation applicative ne peut franchir cette barrière ; observation,
   élections et contrôle de migration continuent.
4. **Dérivation déterministe :** les nœuds formant le quorum vérifient les
   artefacts cibles et dérivent le même candidat migré et la même racine depuis
   la barrière.
5. **`MigrationActivate` (point de commit) :** confirme atomiquement la
   référence de barrière, le profil cible, l'état migré et un **checkpoint
   frontière obligatoire** sous le nouveau profil afin qu'une reprise ordinaire
   ne traverse jamais deux profils d'exécution.
   - Avant activation : un abandon confirmé lève la pause sous l'ancien profil.
   - Après activation : aucun rollback automatique ; revenir en arrière exige
     une nouvelle migration explicite. Les réponses incertaines se résolvent à
     partir de l'histoire confirmée.

### Rétention & sauvegarde (`K-10`, `P1-19`, `P2-02`)

La compaction peut purger l'histoire antérieure aux points de reprise conservés
uniquement en préservant l'état courant, les obligations actives, les
*tombstones* de non-réutilisation / planchers de générations fermées / marques
hautes d'incarnation et les métadonnées de migration. L'archivage intégral est
optionnel. La sauvegarde/restauration à froid et un runbook opérateur de
reprise de base sont obligatoires en v1 ; la sauvegarde à chaud viendra plus
tard.

---

## 8. Observation confirmée

Les observateurs lisent un préfixe confirmé (`K-02`), jamais une queue locale
non confirmée. Chaque réponse identifie `world_id`, `authority_term`,
`config_version`, `confirmed_cursor`, `applied_cursor`, le profil d'exécution
et la fraîcheur :

- **Lecture confirmée courante :** appuyée sur une barrière de lecture par
  quorum dans le terme et la configuration actifs, le nœud serveur ayant
  appliqué jusqu'à cette barrière. Un ancien coordinateur isolé ne peut jamais
  prétendre servir l'état courant.
- **Confirmée connue mais périmée :** explicitement marquée périmée (*stale*) /
  fraîcheur inconnue.

Les flux d'événements portent des IDs d'événements monotones stables (livraison
au moins une fois sur reconnexion ; déduplication côté consommateur). Lorsque
la compaction fait expirer le curseur d'un observateur, le serveur renvoie une
expiration de curseur explicite ainsi qu'un état et un curseur de remplacement
cohérents issus de la même coupe épinglée (en redémarrant explicitement si la
coupe ne peut rester épinglée pendant la bascule).

**Neutralité :** la présence, le nombre, les filtres ou la contre-pression des
observateurs ne modifient jamais les règles canoniques, la cadence de tick, la
fidélité ni les résultats. En surcharge, l'observation est limitée en débit ou
refusée.

---

## 9. Fédération inter-mondes

Seej ne distribue **pas** le tick synchrone d'un même monde sur plusieurs
groupes d'autorité indépendants en v1 (un couplage *lockstep* fait chuter la
disponibilité comme $p^N$). À la place, le passage à l'échelle horizontal et la
décentralisation pair-à-pair entre mondes utilisent des **enveloppes
asynchrones inter-mondes et des obligations durables** :

- Chaque monde avance de façon autonome sous son propre groupe d'autorité CP et
  sa propre horloge de tick.
- Les effets inter-mondes sont des protocoles asynchrones explicites
  (événements *outbox* / obligations de séquestre / commandes entrantes
  validées), jamais des transactions atomiques synchrones inter-mondes.
- Un protocole dédié de transport/fédération P2P inter-mondes est prévu en
  Gate 1-D une fois la v1 mono-monde prouvée.
