# Barnaby — Instructions

## Lire avant de travailler
1. `docs/project/handoff.md` (état courant).
2. Selon la tâche : `docs/SPEC.md` (le produit, il sert de PRD), `docs/project/architecture.md` (frontières et invariants), `DESIGN.md` (interface), le plan en cours dans `docs/plans/`.

La conversation et les décisions récentes du propriétaire priment sur les documents.

## Objectif
App de bureau local-first : dictée au curseur et assistant de réunion (capture micro + système, transcription, diarisation, relecture des zones d'ombre, envoi). Jalon en cours : Phase 1 (MVP local) sur Windows et Linux.

## Stack et commandes
Tauri 2 + Rust (workspace : `crates/core`, `crates/engine`, `src-tauri`), front Svelte 5 + Vite + TypeScript dans `ui/`, avec Bun. Choix et raisons : `docs/project/decisions.md`.

| Tâche | Commande |
|---|---|
| Tests du cœur (partout) | `cargo test -p barnaby-core` |
| Lint Rust | `cargo clippy --workspace --all-targets -- -D warnings` |
| Front : types | `cd ui && bun run check` |
| Front : tests | `cd ui && bun test` |
| App en dev | `cd src-tauri && cargo tauri dev` (machine avec webview et audio) |
| Installeurs | Linux : `cd src-tauri && bunx @tauri-apps/cli build`. Windows : `cargo build --release -p barnaby` puis `bunx @tauri-apps/cli build --config tauri.bundle-windows.json` (DLL sherpa). Ou via la CI. |

Prérequis Linux pour l'app : `libwebkit2gtk-4.1-dev libasound2-dev libxdo-dev libssl-dev cmake clang pulseaudio-utils`. Windows : MSVC, CMake, LLVM.

## Conventions
- Code, identifiants et commits en anglais. Interface en français (anglais disponible), via `ui/src/lib/i18n.ts`.
- Commits courts à l'impératif (`add webhook retries`).
- Les types partagés vivent dans `crates/core/src/model.rs` et sont recopiés à la main dans `ui/src/lib/api.ts`. Modifier les deux dans le même commit.
- Pas de `unwrap()` hors des tests, sauf pour un invariant évident, commenté.
- Journaux : jamais de contenu (texte transcrit, notes, noms). Seulement des identifiants et des codes.

## Interdits et garde-fous
- Pas de requête réseau pendant le traitement en mode `local` (invariant 3 de l'architecture, testé).
- Pas de secret en clair : clé API et secret HMAC vont dans le trousseau de l'OS (crate `keyring`).
- Publier une release, signer ou pousser sur `main` : accord du propriétaire.

## Cycle
`resume-project` → travail → vérifications → `wrap-session`.

- Revue d'interface : `review-ui`. Revue de code : `/code-review` (justesse), `ponytail-review` (simplicité).
