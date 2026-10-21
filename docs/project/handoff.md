# Reprise — Barnaby
Mise à jour : 2026-10-21

## Où on en est
Phase 1 (`docs/plans/phase-1.md`) codée sur la branche `phase-1`, pas encore fusionnée dans `main`. Lots A (cœur), B (moteur), C (interface) et D (app Tauri) sont faits.

Vérifié sur le VPS (Linux, sans matériel audio) :
- `cargo clippy --workspace --all-targets -D warnings` propre ; tests : 26 (cœur), 12 (moteur), 3 (app), 17 (front).
- Pipeline local complet avec de vrais modèles (whisper-tiny + sherpa-onnx, test `e2e` ignoré par défaut) : attribution micro = « me », piste système = autre locuteur, idempotence.
- `.deb` release construit (16 Mo), bibliothèques sherpa résolues via le rpath, app packagée lancée sous Xvfb : la webview s'affiche, l'IPC répond.

## Prochaine action
1. Pousser `phase-1` pour que la CI construise les installeurs Windows et Linux (`.github/workflows/ci.yml`), et corriger ce qui casse sous Windows.
2. Le propriétaire teste sur ses PC (checklist ci-dessous). Ensuite, fusion dans `main` et Phase 0 (mesures DER/WER sur le corpus réel).

## Checklist de test (Windows et Linux)
- Réglages › Modèles : télécharger `whisper-base`, `whisper-small`, `pyannote-segmentation-3-0`, `wespeaker-voxceleb-resnet34-lm`, `silero-vad`.
- Dictée : en mode maintien et en mode bascule, dans un éditeur, un navigateur et un terminal. Vérifier que le texte arrive et que le presse-papiers est restauré. Mesurer la latence.
- Linux : sous X11 (`xdotool`) et sous Wayland (`wtype`). Sans eux, repli sur enigo.
- Réunion : un appel réel (Meet ou Teams), avec niveaux micro et système visibles. Arrêter, attendre la notification, relire, renommer les locuteurs, valider. Contrôler la sortie dossier et webhook.
- Crash : tuer l'app pendant un enregistrement, la relancer. La réunion doit reprendre et se traiter.
- Ollama (facultatif) : activer la structuration avec `http://localhost:11434/v1`.
- CLI : `barnaby --toggle` lance ou arrête une dictée dans l'instance déjà ouverte.

## Bloquants et actions du propriétaire
- Valider le push de `phase-1` (déclenche la CI).
- Logo et chouette définitifs : remplacer `ui/public/owl.svg`, puis `cd src-tauri && bunx @tauri-apps/cli icon ../ui/public/owl.svg` ; régénérer aussi `icons/tray.png` et `icons/tray-active.png`.

## Pièges connus
- VPS de dev sans sudo : sourcer `~/sysroot/env.sh` (en-têtes ALSA/webkit/cmake/clang extraits de .deb) avant toute compilation. Pour lancer les tests de l'app : `LD_LIBRARY_PATH=$HOME/sysroot/root/usr/lib/x86_64-linux-gnu`.
- Les commandes `before*` de `tauri.conf.json` s'exécutent depuis la racine du dépôt, pas depuis `src-tauri`.
- sherpa-onnx est lié dynamiquement : les `.so`/`.dll` sont livrés par `tauri.linux.conf.json` et `tauri.windows.conf.json`. Le binaire Linux cherche dans `$ORIGIN:$ORIGIN/../lib/barnaby` (`build.rs`).
- Pas encore traité :
  - Pas de détection d'un champ texte actif : le texte est toujours collé, et l'historique le garde.
  - Pas de zones « terme incertain ».
  - `injected_into` reste vide sous Windows.
  - L'écho haut-parleur → micro peut fausser l'attribution de « me ».
