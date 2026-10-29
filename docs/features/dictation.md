# Dictée

> Statut : livré · Plan : `docs/plans/phase-1b-dictation.md`

## À quoi ça sert
Dicter dans n'importe quelle application : le texte est transcrit pendant qu'on parle puis collé au curseur. Pour quiconque écrit beaucoup et veut garder sa voix sur sa machine.

## Cas d'usage
1. En tant qu'utilisateur, je maintiens (ou j'active) un raccourci global, je parle, je relâche : le texte apparaît dans le champ actif.
2. Je personnalise un vocabulaire de termes à corriger.

## Règles
Le texte vit dans le presse-papiers quand aucune fenêtre ne peut le recevoir. L'overlay ne prend jamais le focus. Détail : `docs/SPEC.md` section 7 et invariants 4 et 7 de `docs/project/architecture.md`.

## Écrans et points d'entrée
`src-tauri/src/dictation.rs` (orchestration), `src-tauri/src/inject.rs` (collage), `src-tauri/src/overlay.rs`, `src-tauri/src/shortcuts.rs`, `crates/engine/src/live.rs` (flux et VAD), `crates/core/src/vocabulary.rs`.

## Tests
`crates/engine/src/live.rs`, `crates/engine/src/vad.rs`, `crates/core/src/vocabulary.rs` (tests Rust dans les modules). Le collage et le raccourci ne sont validés qu'à la main.

## Limites connues
Linux sans `_NET_ACTIVE_WINDOW` ne colle plus. Les raccourcis ne sont rétablis qu'au `blur`. Aucun test automatique sur un vrai poste.
