# Reprise — Barnaby
Mise à jour : 2026-10-22

## Où on en est
- Branche `phase-1` poussée (`af7dc5f`), CI verte (run 39782732984, installeurs Windows et Linux en Artifacts). `main` ne contient que la spec.
- Jon a testé le build sur Windows (onboarding, dictée, réunion Mistral) : tout fonctionne.
- Fait cette session :
  - transcription des réunions en ligne avec Mistral Voxtral (plan `docs/plans/cloud-transcription.md`, spec `docs/project/cloud-transcription.md`) ;
  - refonte visuelle « Things, version chouette » et onboarding au premier lancement (plan `docs/plans/redesign-onboarding.md`, spec `docs/project/redesign-onboarding.md`, système dans `DESIGN.md`, références dans `design/references/refs.md`, produit dans `PRODUCT.md`).
- Chaque tâche a eu sa revue ; revue finale de branche, revue visuelle (impeccable) et revue d'accessibilité (review-ui) faites, corrections appliquées et revérifiées.

## Prochaine action
1. Logo : icône d'app et petit logo de la barre latérale tirés de la chouette (découpe propre de la tête dans `design/references/owl-sheet.png`, ou image propre fournie par Jon), puis `cd src-tauri && bunx @tauri-apps/cli icon <fichier>` et `icons/tray*.png`.
2. Barre de titre : retirer la barre système (`decorations: false` sous Windows et Linux, barre transparente sous macOS), barre latérale sur toute la hauteur avec chouette + « Barnaby », zones de glissement, boutons réduire/agrandir/fermer redessinés (Snap Layouts de Windows 11, double-clic).
3. Onboarding, étape Modèles : badge « Recommandé pour ton ordinateur » (local ou Mistral) selon la mémoire et le nombre de cœurs.
4. Son de fin de traitement : renommer le réglage en « Jouer un son quand une réunion est prête à relire », puis tester avec une vraie réunion ; si muet sous Windows, corriger le nom du son (`n.sound("default")` dans `src-tauri/src/meeting.rs`).
5. Site : landing page très simple qui présente le produit (passer par `kickoff-project` pour le cadrage, puis `choose-stack`).
6. Ensuite : PR `phase-1` → `main` (accord de Jon pour fusionner).

## Vérifications (2026-10-22, VPS, sur `bdcade5`)
- `cd ui && bun run check` : 0 erreur, 0 avertissement. `bun test` : 40 ok, 0 échec.
- `. ~/sysroot/env.sh && cargo clippy --workspace --all-targets -j2 -- -D warnings` : propre.
- `LD_LIBRARY_PATH=$HOME/sysroot/root/usr/lib/x86_64-linux-gnu cargo test --workspace -j2` : 44 (cœur), 17 (moteur, 6 ignorés), 3 (app), tous verts.
- `impeccable detect --json ui/src` : aucune alerte. Captures de tous les écrans (clair, sombre, 1440 et 900 px) dans `design/shots/` (non versionné).
- Non vérifié : aucun appel réel à Mistral ; aucun test dans la vraie fenêtre Tauri (WebView2, WebKitGTK) : raccourci global, collage dans la zone de test, curseurs et listes déroulantes stylés ; animations vues seulement en mouvement réduit.

## État non commité
- `barnaby.zip` à la racine (dépôt de Jon, non versionné ; son contenu est copié dans `design/references/`).

## Bloquants et actions du propriétaire
- Logo : dire si la découpe de la planche suffit ou envoyer une image propre.
- Chouette : fournir une version détourée (fond transparent, sans filigrane Kling) de la boucle vidéo pour l'onboarding ; en attendant, l'app utilise des découpes de la planche. Logo final et nouvelle icône d'app à fournir.
- À trancher : sélecteur de micro dans l'onboarding (aujourd'hui le micro par défaut du système) ; tour des fonctions en démonstration animée plutôt qu'en trois panneaux ; bouton « Retranscrire » pour refaire une ancienne réunion avec Mistral.
- Outil impeccable : le tirage de direction n'a pas imprimé de « seed reference » ; le contrat garde `seed: none (unavailable)`.

## Pièges connus
- VPS sans sudo : sourcer `~/sysroot/env.sh` avant toute compilation ; un seul cargo à la fois (2 CPU) ; jamais `pkill -f vite` ni `cargo fmt` (le dépôt n'est pas formaté, il réécrit 20 fichiers).
- Captures : `bun run dev` (mode navigateur avec `ui/src/lib/mock.ts`), Playwright depuis `~/.npm/_npx/fd3bca3c548369c0/node_modules/playwright`.
- Les commandes `before*` de `tauri.conf.json` s'exécutent depuis la racine du dépôt. sherpa-onnx lié dynamiquement (rpath `$ORIGIN/../lib/barnaby`). Overlay Windows : ne jamais le cacher, seulement le déplacer.
- Pas encore traité : AppImage, `injected_into` vide sous Windows, écho haut-parleur qui fausse « me », ~1 Go de RAM par heure de réunion, sur-correction du vocabulaire, purge des 7 jours faite seulement au démarrage et à l'envoi, téléchargement de modèle bloqué si le serveur cesse d'envoyer en plein fichier (on peut passer l'étape).
