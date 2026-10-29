# Réunion : capture et traitement

> Statut : livré · Plan : `docs/plans/meeting-visibility.md`

## À quoi ça sert
Enregistrer une réunion (micro et son du système), la transcrire et séparer les voix, sans rien envoyer hors de la machine en mode local.

## Cas d'usage
1. Je démarre une réunion avec ses participants, j'ajoute des notes pendant qu'elle dure, j'arrête.
2. Je vois la progression du traitement sur l'accueil et dans le journal ; je relance une réunion en échec.

## Règles
Chaque étape est idempotente et reprend après un redémarrage (invariant 2 d'architecture). L'audio est écrit en continu (invariant 1). Détail : `docs/SPEC.md` sections 8 et 9.

## Écrans et points d'entrée
`src-tauri/src/meeting.rs`, `crates/core/src/pipeline.rs`, `crates/engine/src/capture.rs` (micro, loopback), `crates/engine/src/diarize.rs`, `ui/src/screens/Home.svelte`, `ui/src/screens/Recording.svelte`.

## Tests
`crates/core/src/pipeline.rs`, `crates/engine/src/capture.rs`, `crates/engine/src/e2e.rs`, `crates/engine/src/diarize.rs` (le test de diarisation est `--ignored`).

## Limites connues
Séparation des voix limitée à un cœur et lente sur une longue réunion. Réunion à un seul micro : tout le monde est étiqueté « Moi ». Écho des haut-parleurs non traité. macOS jamais testé sur un vrai Mac.
