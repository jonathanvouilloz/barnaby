# Reprise — Barnaby
Mise à jour : 2026-10-22

## Où on en est
- Phase 1 (`docs/plans/phase-1.md`) et itération 1b (`docs/plans/phase-1b-dictation.md`) codées sur `phase-1`, poussées. `main` ne contient que la spec. Pas de PR : la fusion attend les tests du propriétaire.
- Test Windows du propriétaire : Google Meet n'était pas enregistré dans la piste système (YouTube l'était). Corrigé dans `a44a0de` : capture de toutes les sorties actives, mélangées, avec exclusion possible dans Réglages › Réunion (`docs/plans/system-audio-all-outputs.md`, décision du 2026-10-22).
- Les correctifs de `e0ee496` (téléchargement Parakeet en boucle, fenêtre « Modèles à installer » bloquée, bulle de dictée visible au démarrage) ne sont toujours pas revérifiés sur Windows.
- Installeurs de `a44a0de` (Windows `.exe`/`.msi`, Linux `.deb`) : https://github.com/jonathanvouilloz/barnaby/actions/runs/39749837986 (section Artifacts).
- Mascotte : orientation choisie (dernière entrée mascotte de `decisions.md`). Le propriétaire génère l'image puis la vidéo.

## Prochaine action
1. Le propriétaire installe ce build et teste une réunion Meet : avec casque Bluetooth, puis avec haut-parleurs, YouTube en même temps. Puis il décoche une sortie dans Réglages › Réunion et vérifie qu'elle n'est plus enregistrée.
2. Si Meet manque encore : lire les logs (`output captured:` / `output closed:` / `capture failed`) ; plan B = capture par processus Windows.
3. Il continue la checklist ci-dessous et rapporte. On corrige, puis PR `phase-1` → `main` (accord requis pour fusionner).
4. Vidéo de la mascotte (`scp` à la racine du dépôt) : WebM et MP4 800 px (site), WebM 512 px (onboarding).
5. Ensuite : build GPU (Vulkan) de whisper, puis Phase 0 (DER/WER sur le corpus réel).

## Vérifications (2026-10-22, VPS, sur `a44a0de`)
- `. ~/sysroot/env.sh && cargo clippy --workspace --all-targets -j2 -- -D warnings` : propre.
- `LD_LIBRARY_PATH=$HOME/sysroot/root/usr/lib/x86_64-linux-gnu cargo test --workspace -j2` : 31 (cœur), 17 (moteur, 6 ignorés), 4 (app), tous verts.
- `cd ui && bun run check && bun test` : 0 erreur, 22 tests verts.
- Code Windows de `outputs.rs` : `cargo check --target x86_64-pc-windows-msvc` dans un crate de test du scratchpad, compile. CI run 39749837986 : succès (Windows et Ubuntu).
- Non vérifié : aucune capture réelle (le VPS n'a ni carte son ni `pactl`) ; correctifs de `e0ee496` sur Windows ; OpenRouter avec une vraie clé.

## Checklist de test (Windows, puis Linux)
- Dicter dans le Bloc-notes : le texte doit y arriver (l'overlay ne vole pas le focus).
- Réglages d'avant la 1b : remettre Parakeet v3 et le mode Bascule dans Réglages › Dictée.
- Dictée de 30 à 60 s avec des pauses : latence, ponctuation, vocabulaire (« Barber Concept »).
- Réunion réelle (Meet ou Teams) : niveaux, fin, relecture, locuteurs, envoi dossier et webhook. Comparer Whisper small (défaut) et large-v3 turbo.
- Résumé cloud : Réglages › Structuration (IA) › Cloud (OpenRouter), clé, identifiant de modèle (ex. `deepseek/deepseek-v4-flash`).
- Crash pendant un enregistrement, puis relance : la réunion reprend. `barnaby --toggle` depuis un terminal.
- Logs : `%LOCALAPPDATA%\app.barnaby.desktop\logs` (chercher `download_failed:` ou `capture failed`).

## État non commité
Rien. Branche `phase-1`.

## Bloquants et actions du propriétaire
- Tester le build et rapporter.
- Mascotte : choisir l'image, générer la vidéo, l'envoyer. Dire si on retire les six états de la chouette (spec 10.4 et `Owl.svelte`).
- Nouvelle icône d'app : fournir un SVG simplifié, puis `cd src-tauri && bunx @tauri-apps/cli icon <fichier>.svg` et refaire `icons/tray*.png`.

## Pièges connus
- VPS sans sudo : sourcer `~/sysroot/env.sh` avant toute compilation ; un seul cargo à la fois (2 CPU) ; jamais `pkill -f vite` (tue le shell), utiliser `kill <pid>`.
- Les commandes `before*` de `tauri.conf.json` s'exécutent depuis la racine du dépôt.
- sherpa-onnx lié dynamiquement : rpath `$ORIGIN/../lib/barnaby` sous Linux ; `tauri.bundle-windows.json` seulement à l'empaquetage Windows ; pas de rust-cache dans le job de build de la CI.
- Overlay Windows : ne jamais le cacher ni lui donner de position de départ hors écran, seulement le déplacer (voir `decisions.md`).
- Capture système : une sortie exclue puis débranchée s'affiche sous son identifiant brut ; l'alerte `capture.no_system` reste affichée si aucune sortie n'existait au démarrage.
- Pas encore traité : AppImage, zones « terme incertain », bouton « régénérer le résumé », `injected_into` vide sous Windows, écho haut-parleur qui fausse « me », environ 1 Go de RAM par heure de réunion, sur-correction du vocabulaire (« barbier concept »).
