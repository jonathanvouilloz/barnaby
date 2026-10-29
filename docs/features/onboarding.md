# Premier lancement

> Statut : livré · Plan : `docs/plans/redesign-onboarding.md`

## À quoi ça sert
Guider une installation neuve : micro, modèles, essai de dictée, dossier de destination, visite de l'interface.

## Cas d'usage
1. À la première ouverture, je suis les étapes jusqu'à une dictée réussie.

## Règles
Cadrage visuel : `docs/project/redesign-onboarding.md` et `DESIGN.md`.

## Écrans et points d'entrée
`ui/src/onboarding/Onboarding.svelte`, `ui/src/onboarding/steps.ts`.

## Tests
`ui/src/lib/lib.test.ts`.

## Limites connues
Pas de sélecteur de micro dans l'onboarding.
