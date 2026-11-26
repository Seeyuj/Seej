# Benchmarks empiriques (preuves de laboratoire D-015)

> *Miroir français de [`seej/docs/BENCHMARKS.md`](../../seej/docs/BENCHMARKS.md).
> La version anglaise fait foi en cas d'écart.*

```text
status: RÉFÉRENCE EMPIRIQUE — MESURÉE SUR HARNAIS JETABLE AVANT RÉÉCRITURE DU CŒUR
```

Pour ancrer [D-015](DECISIONS.md#d-015--noyau-mesuré-et-couture-plugin-gelée)
dans des mesures réelles avant d'écrire le noyau v1 sur base vierge, deux
campagnes autonomes mono-thread (`seej_lab_workloads` sur les branches
`lab/wal-fail-closed-load-b` `1c98282` et `lab/state-layout-b` `5856396`, hors
workspace/CI) ont évalué trois charges synthétiques.

---

## Environnement de test et charges

- **Matériel / chaîne :** AMD Ryzen 9 5900X + NVMe Crucial P2 1 To, Windows 11,
  `rustc 1.95.0 --release`, exécution mono-thread.
- **Taille d'état :** **1 000 000 clés × 128 o** (1 000 partitions × 1 000
  enregistrements ; checkpoint ~144 Mo).
- **Cadence cible :** **budget de tick 10 Hz / 100 ms**.
- **Vérification de reprise :** **24/24 reprises** (checkpoint au tick 100 +
  200 ticks de rejeu) ont reproduit exactement la racine d'état sémantique de
  l'exécution directe.

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

| Charge & variante | `delta` p99 (ms) | `root` p99 (ms) | `delta` octets/tick | `root` octets/tick | Meilleur rejeu (ms/tick) |
| --- | ---: | ---: | ---: | ---: | ---: |
| **A** — `btree/acc` | 10,0–10,8 | 8,8–9,2 | 417 Ko (4,2 Mo/s) | 129 Ko (1,3 Mo/s) | 2,78 (`delta`) |
| **A** — `partitioned/acc-cached` | **8,6** | **7,6** | 417 Ko | 129 Ko | **2,06** (`delta`) |
| **A** — `partitioned/rehash` | 162,6 | 166,8 | 417 Ko | 129 Ko | — (cassé par le rehash par cmd) |
| **C** — `btree/acc` | 11,8–12,4 | 10,7–11,7 | 561 Ko (5,6 Mo/s) | 129 Ko | 3,35 (`delta`) |
| **C** — `partitioned/acc-cached` | 21,7* (p50 **8,4**) | 29,4* (p50 **7,6**) | 561 Ko | 129 Ko | **~2,5–3,7** |
| **B** — `btree/acc` | 161–445 | 140–147 | **14,4 Mo (144 Mo/s)** | **89 o** | 113–129 (ratio > 1,1) |
| **B** — `partitioned/acc` | 130–348 | **95,1** | 14,4 Mo | 89 o | 70–82 |
| **B** — `partitioned/acc-cached` | 279–383 | **79,0** (p50 **66,1**) | 14,4 Mo | 89 o | **54,6** (`delta`) / **61,8** (`root`) |
| **B** — `partitioned/rehash` | 113–483 | 98,5 | 14,4 Mo | 89 o | 73,7–77,7 |

*\*Sur C (`partitioned/acc-cached`), toutes les phases CPU sont restées ≤ 7,6 ms max ; le saut du p99 provient de deux pointes de latence NVMe `sync_data` de 50–60 ms décalées au rang 288/290 par le checkpoint au tick 100.*

---

## Enseignements empiriques guidant D-015

1. **Les charges de commandes et mixtes (A et C) passent avec une marge ~10× en
   mode `delta`** sur un seul cœur, validant `delta` comme valeur par défaut
   sûre en v1 tant que les plugins au démarrage restent en Rust natif non isolé.
2. **La charge autonome dense B échoue en `delta` sur la latence de queue
   disque** (14,4 Mo/tick d'écritures synchrones donnant un p99 jusqu'à
   283–400 ms et 144 Mo/s avant réplication), confirmant que le mode de preuve
   (`delta` vs futur `root`) doit vivre dans le profil d'exécution versionné
   plutôt que d'être codé en dur.
3. **Le stockage contigu par partition domine le coût CPU en charge dense :**
   il réduit le p99 de B en `root` de **147 ms à 79 ms** sur un thread. À
   l'inverse, le rehash complet par partition détruit les charges de commandes
   dispersées (**115–169 ms** sur A/C) lorsque des racines par résolution sont
   maintenues — prouvant que les racines doivent se mettre à jour de façon
   incrémentale par enregistrement modifié.
4. **Les checkpoints complets synchrones (144 Mo) bloquent chaque charge
   pendant 237–435 ms** (une pointe à 1 485 ms), confirmant que des checkpoints
   CoW asynchrones hors chemin critique sont nécessaires avant le passage à
   grande échelle en production (Phase 4.1 / Gate 1-D).
5. **Limites de ces mesures :** mono-thread, état en RAM, accumulateur BLAKE3
   additif expérimental (pas un schéma cryptographique de production), cache OS
   chaud à la reprise, pas de surcoût réseau/quorum et aucun moteur embarqué
   (`redb`/`Fjall`) encore mesuré — les vrais benchmarks de moteur de stockage
   auront lieu au démarrage de la **Phase 1.4** ([ROADMAP.md](ROADMAP.md)).
