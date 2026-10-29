# Relecture des zones d'ombre

> Statut : livré · Plan : `docs/plans/phase-1.md`

## À quoi ça sert
Corriger les passages douteux d'une transcription (faible confiance, voix superposées, locuteur inconnu) avant tout envoi.

## Cas d'usage
1. Je parcours la file des zones à relire, j'écoute l'extrait, je corrige ou j'écarte.
2. Je renomme un locuteur ; la modification vaut pour toute la réunion.
3. Je valide : la réunion part vers sa destination.

## Règles
Une réunion validée s'ouvre ensuite en lecture seule. Détail : `docs/SPEC.md` section 13.

## Écrans et points d'entrée
`crates/core/src/review.rs`, `ui/src/screens/Review.svelte`, `ui/src/screens/ReviewQueue.svelte`, `ui/src/screens/ReviewZone.svelte`, `ui/src/lib/review.ts`.

## Tests
`crates/core/src/review.rs`, `ui/src/lib/lib.test.ts`.

## Limites connues
En lecture seule, les zones traitées restent surlignées. L'audio est supprimé à la validation.
