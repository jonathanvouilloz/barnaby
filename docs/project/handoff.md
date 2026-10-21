# Reprise — Barnaby
Mise à jour : 2026-10-21

## Où on en est
- Phase 1 (`docs/plans/phase-1.md`) et itération 1b (`docs/plans/phase-1b-dictation.md`) codées sur la branche `phase-1`, poussées. `main` ne contient que la spec. Aucune PR ouverte : le propriétaire fusionnera après ses tests.
- Dictée : Parakeet v3 transcrit pendant la parole, en mode bascule (Ctrl+Alt+Espace), avec overlay, vocabulaire et collage au curseur hors historique du presse-papiers.
- Réunion : capture micro et système, whisper + sherpa, relecture, export dossier et webhook. Résumé IA local (Ollama) ou cloud (OpenRouter, mode `hybrid`).
- Installeurs de la CI, dernier run vert (Windows `.exe`/`.msi`, Linux `.deb`) : https://github.com/jonathanvouilloz/barnaby/actions/runs/39625164580 (section Artifacts).

## Vérifications faites (2026-10-21, VPS)
- `. ~/sysroot/env.sh && cargo clippy --workspace --all-targets -j2 -- -D warnings` : propre.
- `LD_LIBRARY_PATH=$HOME/sysroot/root/usr/lib/x86_64-linux-gnu cargo test --workspace -j2` : 31 tests (cœur), 13 (moteur, dont 6 ignorés), 4 (app), tous verts.
- `cd ui && bun run check && bun test` : 0 erreur, 22 tests verts.
- `BARNABY_MODELS_DIR=$PWD/target/test-models cargo test --release -p barnaby-engine -- --ignored` :
  - pipeline réunion de bout en bout : OK ;
  - dictée de 62 s : 0,47 s entre l'arrêt et le texte ;
  - chargement de Parakeet : environ 6 à 8 s.
- `.deb` packagé et lancé sous Xvfb : webview et IPC OK. C'était avant la 1b, l'overlay n'a pas été vu dans une vraie fenêtre.
- Rien n'a été testé avec du vrai matériel audio. Arbre de travail propre.

## Prochaine action
1. Le propriétaire teste l'installeur Windows (checklist ci-dessous) et rapporte : la latence ressentie, le collage, le raccourci, l'overlay.
2. Corriger d'après son retour, puis ouvrir la PR `phase-1` → `main`. La fusion attend son accord.
3. Ensuite : build GPU (Vulkan) de whisper pour les réunions, Phase 0 (DER/WER sur le corpus réel).

## Checklist de test (Windows, puis Linux)
- **D'abord** : dicter dans le Bloc-notes. Le texte doit y arriver, ce qui prouve que l'overlay ne vole pas le focus.
- Premier lancement : la feuille « Modèles à installer », puis « Tout télécharger ». Réglages enregistrés avant la 1b : remettre Parakeet v3 et le mode Bascule dans Réglages › Dictée.
- Dictée de 30 à 60 s avec des pauses : latence, ponctuation, vocabulaire (« Barber Concept »).
- Réunion réelle (Meet ou Teams) : niveaux micro et système, notification de fin, relecture, renommage des locuteurs, envoi vers le dossier et le webhook.
- Résumé cloud : clé OpenRouter, un modèle (`qwen/…`, `deepseek/…`, `z-ai/glm-…`).
- Crash pendant un enregistrement, puis relance : la réunion doit reprendre. `barnaby --toggle` depuis un terminal.
- Logs : `%LOCALAPPDATA%\app.barnaby.desktop\logs`.

## Bloquants et actions du propriétaire
- Tester l'installeur et rapporter les problèmes.
- Fournir le logo et la chouette définitifs : remplacer `ui/public/owl.svg`, puis `cd src-tauri && bunx @tauri-apps/cli icon ../ui/public/owl.svg`, et refaire `icons/tray*.png`.

## Pièges connus
- VPS sans sudo :
  - Sourcer `~/sysroot/env.sh` avant toute compilation.
  - Lancer un seul cargo à la fois (2 CPU).
  - Ne jamais faire `pkill -f vite` : la commande tue aussi le shell qui la lance. Utiliser `kill <pid>`.
- Les commandes `before*` de `tauri.conf.json` s'exécutent depuis la racine du dépôt.
- sherpa-onnx est lié dynamiquement :
  - Linux : rpath `$ORIGIN/../lib/barnaby` et `tauri.linux.conf.json`.
  - Windows : `tauri.bundle-windows.json` est passé seulement à l'empaquetage, après `cargo build --release` (voir AGENTS.md).
  - Pas de rust-cache dans le job de build de la CI : il restaure sherpa sans ses bibliothèques.
- Pas encore traité :
  - Pas d'AppImage.
  - Pas de zones « terme incertain ».
  - Pas de bouton « régénérer le résumé ».
  - `injected_into` reste vide sous Windows.
  - L'écho haut-parleur peut fausser l'attribution de « me ».
  - Environ 1 Go de RAM par heure de réunion.
  - Le vocabulaire peut sur-corriger un mot proche (« barbier concept » devient « Barber Concept »).
