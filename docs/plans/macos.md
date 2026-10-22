# Plan — version macOS

Objectif : un `.dmg` pour Mac Apple Silicon (macOS 14.2 et plus), publié par la CI avec les autres installeurs, et le site qui le propose. Sans Mac sous la main : la CI (runner macOS) compile et teste ; la capture réelle, l'injection et les permissions restent à valider sur un vrai Mac.

## Tâches

1. **Son de l'ordinateur** (`crates/engine/src/outputs.rs`) : sous macOS, une seule source, un « process tap » global de Core Audio (tout le son de tous les processus, stéréo) branché sur un agrégat privé, lu par cpal comme une entrée. Pas de capture par sortie : cpal enregistrerait le micro d'un casque qui a aussi une entrée (AirPods).
2. **Dépendances macOS** : sherpa-onnx en statique (rien à embarquer dans le `.app`), whisper.cpp avec Metal.
3. **App** (`src-tauri`) : `Info.plist` (micro, capture audio), `tauri.macos.conf.json` (cibles `app` + `dmg`, macOS 14.2 minimum, signature ad hoc, API privée pour l'overlay transparent), réouverture par le Dock, demande d'accès Accessibilité au lancement (collage).
4. **Raccourcis** : défauts macOS (⌘⇧Espace, ⌘⇧R, ⌘⇧M, ⌘⇧L), libellés ⌘ ⌥ ⌃ dans l'interface.
5. **CI** : job macOS (`macos-14`, arm64), tests du cœur et du moteur, `.dmg` en artefact ; la release publie `Barnaby-macos-arm64.dmg`.
6. **Site** : bouton Mac, lien direct, questions mises à jour.
7. **Docs et livraison** : décisions, architecture, handoff ; fusion `phase-1` → `main`, tag `v0.2.0`.

## Vérification

- Local : `cargo check --target aarch64-apple-darwin` sur le module macOS isolé (pas de SDK Apple ici), tests et clippy Linux, `bun run check` et `bun test`.
- CI : les trois jobs de build verts, `.dmg` présent dans la release.
- Jon, sur un Mac : installer, dicter, enregistrer une réunion avec du son système.
