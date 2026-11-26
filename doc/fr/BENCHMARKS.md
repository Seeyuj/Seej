# Benchmarks empiriques (preuves de laboratoire D-015)

> **Traduction non normative** de [`seej/docs/BENCHMARKS.md`](../../seej/docs/BENCHMARKS.md).
> La version anglaise fait foi en cas d'écart.

```text
status: RÉFÉRENCE EMPIRIQUE — MESURÉE SUR HARNAIS JETABLE AVANT RÉÉCRITURE DU CŒUR
```

Pour ancrer [D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée)
dans des mesures réelles avant d'écrire le noyau v1 sur base vierge, deux
campagnes autonomes mono-thread (`seej_lab_workloads`, hors workspace/CI) ont
évalué trois charges synthétiques. Les deux harnais bruts, rapports et résumés
JSON sont conservés de façon permanente aux tags d'archive Git :

- **Campagne 1 :** tag `archive/lab-campaign-1` (`1c98282`, `seej/server/lab/workloads/RESULTS.md`) — 6 séries (témoin `BTreeMap` × `delta`/`root`), **6/6 reprises propres**.
- **Campagne 2 :** tag `archive/lab-campaign-2` (`5856396`, `seej/server/lab/workloads/RESULTS-layout.md`) — 24 séries (4 variantes stockage/hash + resérie B/`delta` en ordre inverse), **24/24 reprises propres**.

---

## Environnement de test et charges

- **Matériel / chaîne :** AMD Ryzen 9 5900X + NVMe Crucial P2 1 To, Windows 11,
  `rustc 1.95.0 --release`, exécution mono-thread.
- **Taille d'état :** **1 000 000 clés × 128 o** (1 000 partitions × 1 000
  enregistrements ; checkpoint 144 000 068 octets).
- **Cadence cible :** **budget de tick 10 Hz / 100 ms** (300 ticks, 10
  d'échauffement, 290 mesurés, checkpoint complet au tick 100).
- **Vérification de reprise :** chaque reprise propre (checkpoint au tick 100 +
  200 ticks rejoués, cache OS non purgé — température effective du cache
  inconnue — et **sans injection de pannes**) a reproduit exactement la racine
  d'état sémantique de son exécution directe (`6/6` en Campagne 1 ; `24/24` en
  Campagne 2).

Charges testées :

- **Charge A (commandes) :** 1 000 commandes/tick (10 000 cmd/s), 0 écriture
  autonome.
- **Charge B (autonome dense) :** 100 000 écritures autonomes/tick
  (1 M écritures/s), 0 commande.
- **Charge C (mixte + points chauds) :** 1 000 commandes + 1 000 écritures
  autonomes/tick, 80 % frappant 32 clés chaudes.

Variantes stockage & hachage testées :

- `btree/acc` : `BTreeMap` + overlay + accumulateur BLAKE3 additif incrémental.
- `partitioned/acc` : stockage contigu par partition avec journal d'annulation
  en place + accumulateur incrémental.
- `partitioned/acc-cached` : stockage contigu par partition + empreintes par
  enregistrement mises en cache (1 BLAKE3 par écriture au lieu de 2).
- `partitioned/rehash` : stockage contigu par partition + rehash complet par
  partition touchée.

---

## Résultats mesurés

| Charge & variante | `delta` p99 (ms) | `root` p99 (ms) | `delta` octets/tick | `root` octets/tick | Rejeu (ms/tick) & ratio (/100 ms) |
| --- | ---: | ---: | ---: | ---: | ---: |
| **A** — `btree/acc` | 10,01–10,83 | 8,82–9,20 | 417 Ko (4,17 Mo/s) | 129 Ko (1,29 Mo/s) | 2,78 (`delta`, 0,028) |
| **A** — `partitioned/acc-cached` | **8,62** | **7,55** | 417 Ko | 129 Ko | **2,06** (`delta`, 0,021) |
| **A** — `partitioned/rehash` | 162,58 | 166,81 | 417 Ko | 129 Ko | 106,30 `delta` (1,06) / 109,19 `root` (1,09) |
| **C** — `btree/acc` | 11,77–12,36 | 10,72–11,69 | 561 Ko (5,61 Mo/s) | 129 Ko | 3,35 (`delta`, 0,034) |
| **C** — `partitioned/acc-cached` | 21,74* (p50 **8,38**) | 29,42* (p50 **7,59**) | 561 Ko | 129 Ko | **2,75** `delta` (0,028) / **3,74** `root` (0,037) |
| **B** — `btree/acc` | 161,13–444,59 | 139,69–147,07 | **14,4 Mo (144 Mo/s)** | **89 o** | 113,12–129,44 (ratio **1,13–1,29**) |
| **B** — `partitioned/acc` | 129,64–348,07 | **95,13** | 14,4 Mo | 89 o | 69,85–82,34 (ratio **0,70–0,82**) |
| **B** — `partitioned/acc-cached` | 279,18–383,31 | **78,95** (p50 **66,13**) | 14,4 Mo | 89 o | **54,62** `delta` (**0,55**) / **61,79** `root` (**0,62**) |
| **B** — `partitioned/rehash` | 113,41–482,78 | 98,48 | 14,4 Mo | 89 o | 73,65–77,71 (ratio **0,74–0,78**) |

*\*Sur C (`partitioned/acc-cached`), le p99 est monté à 21,74 ms (`delta`) et 29,42 ms (`root`) alors que la médiane baissait (`8,38` / `7,59 ms`). Comme consigné dans `RESULTS-layout.md`, cela a été mesuré une seule fois et n'est pas expliqué par ces mesures.*

---

## Enseignements empiriques guidant D-015

1. **Les charges de commandes et mixtes (A et C) passent en mode `delta` avec
   une marge de 4,6×–11,6×** sur un seul cœur (`p99 = 8,62–21,74 ms`, médiane
   `6,64–9,95 ms`), validant `delta` comme valeur par défaut sûre en v1 tant
   que les plugins au démarrage restent en Rust natif non isolé.
2. **La charge autonome dense B échoue en `delta` sur les 9 exécutions** (1 en
   Campagne 1 + 8 en Campagne 2) : le p99 du tick complet va de **113,41 à
   482,78 ms** (tiré par la latence variable du `sync_data` de 14,4 Mo/tick
   jusqu'à **283,10–400,12 ms** p99, max **689,37 ms**, et 144 Mo/s projetés
   avant réplication), confirmant que le mode de preuve (`delta` vs futur
   `root`) doit vivre dans le profil d'exécution versionné.
3. **Le stockage contigu par partition domine le coût CPU en charge dense, mais
   la cible de ratio de rejeu `< 0,5` sur B n'est PAS atteinte :** ce layout
   réduit le p99 de B en `root` de **147,07 ms à 78,95 ms** sur un thread, mais
   aucune variante testée n'atteint `< 0,5` (meilleur : **0,55** en `delta` /
   **0,62** en `root`). À l'inverse, le rehash complet par partition détruit
   les charges de commandes dispersées (**115–169 ms** médiane sur A/C) —
   prouvant que les racines doivent se mettre à jour incrémentalement par
   enregistrement modifié.
4. **Les checkpoints complets synchrones (144 Mo) bloquent le tick 100 pendant
   237–435 ms** sur la Campagne 2 (une pointe à **1 485 ms** ; Campagne 1 :
   **247–326 ms**), confirmant que des checkpoints CoW asynchrones hors chemin
   critique sont nécessaires à grande échelle (Phase 4.1 / Gate 1-D).
5. **Limites de ces mesures :** mono-thread, état en RAM, accumulateur BLAKE3
   additif expérimental (pas un schéma cryptographique de production), rejeu
   propre avec cache OS non purgé (température effective du cache inconnue) sans
   injection de pannes, pas de surcoût réseau/quorum et aucun moteur embarqué
   (`redb`/`Fjall`) encore mesuré — les vrais benchmarks de moteur de stockage
   auront lieu au démarrage de la **Phase 1.4** ([ROADMAP.md](ROADMAP.md)).
