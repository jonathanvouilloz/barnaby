# Décisions — Barnaby

Les décisions produit (sections 19 et 22) sont dans `docs/SPEC.md`. Ici, seulement les choix d'implémentation.

## 2026-10-21 — Frontend : Svelte 5 + Vite, sans SvelteKit
- Choix : Svelte 5 (runes) + Vite + TypeScript, construit avec Bun, dans la webview Tauri.
- Écarté : React (comme Handy : runtime plus lourd, moins familier pour le propriétaire), TS vanilla (état et composants réécrits à la main), SvelteKit (son routeur et son SSR ne servent à rien dans une app de bureau).
- Pourquoi : runtime quasi nul (objectif app < 50 Mo, démarrage < 3 s), stack principale du propriétaire, qui a demandé « la plus efficiente ».
- À rouvrir si : on veut réutiliser des composants de Handy tels quels.

## 2026-10-21 — Phase 1 : Windows et Linux, sans passer par la Phase 0
- Choix : MVP local (spec section 21, Phase 1) sur Windows x64 et Linux. macOS compile mais n'est pas validé.
- Pourquoi : le propriétaire teste sur Windows et Linux. Les mesures de la Phase 0 (DER, WER) se font sur son PC, avec le corpus réel.
- À rouvrir si : il faut une version macOS.

## 2026-10-21 — Workspace en trois crates Rust
- Choix : `barnaby-core` (logique pure, testée partout), `barnaby-engine` (audio, whisper, sherpa : dépendances natives), `src-tauri` (app).
- Pourquoi : le VPS de développement n'a ni ALSA ni webkit2gtk. Le cœur se teste partout, le natif se compile en CI (GitHub Actions, Windows + Ubuntu).
- À rouvrir si : la frontière entre les crates finit par coûter plus d'indirection qu'elle ne fait gagner.

## 2026-10-21 — Audio système Linux via `parec`
- Choix : sous Linux, la piste système est lue avec `parec --device=@DEFAULT_MONITOR@` (pipewire-pulse ou PulseAudio) ; sous Windows, avec un loopback WASAPI via cpal.
- Écarté : bindings libpulse/libpipewire (une dépendance native de plus pour lire un flux PCM).
- Pourquoi : `parec` est présent sur Ubuntu 22.04+ (pipewire-pulse) et fonctionne avec les deux serveurs.
- À rouvrir si : `parec` manque sur une distribution cible.

## 2026-10-21 — Design : brief « Apple-like » de la spec, sans phase de références
- Choix : `DESIGN.md` minimal, avec la police système et des tokens clair/sombre, tirés de la spec (section 10.1).
- Pourquoi : le propriétaire a voulu avancer sur le build. La spec fixe déjà la direction et la typographie système.
- À rouvrir si : une refonte visuelle est demandée (passer alors par `design-references` puis `/impeccable`).
