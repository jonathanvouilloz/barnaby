# Reprise : Barnaby
Mise à jour : 2026-11-23

## Où on en est
- Dernière release : `v0.4.6` (GitHub ; site à redéployer). Déchargement des modèles après 10 min sans usage (`MODEL_IDLE_UNLOAD`, `state.rs`), à confirmer par Jon sur la mémoire réelle. `v0.4.5` Contient la PR #2 (Parakeet, portails Wayland, plantage Linux) et la PR #3 (collage Shift+Insert, XWayland sur GNOME) → `docs/features/linux-terminal-paste-focus.md`.
- Testé par Jon (Fedora 44, GNOME Wayland, échelle 100 %) : dictée collée dans Ptyxis et VS Code, focus gardé.

## Prochaine action
1. Confirmer avec Jon que l'autorisation « bureau à distance » n'est demandée qu'une fois, y compris après redémarrage de Barnaby.
2. Tests Windows et `.dmg` sur un vrai Mac toujours en attente (voir Bloquants).

## Vérifications (2026-11-16)
- `cargo clippy -p barnaby` propre sous Linux ; CI du tag `v0.4.6` verte sur les trois OS.
- Non vérifié : flou XWayland en échelle fractionnaire chez d'autres utilisateurs ; pastille et focus sur Mac.

## État non commité
- Rien.

## Bloquants et actions du propriétaire
- Le jeton Cloudflare du serveur (`CLOUDFLARE_API_TOKEN`) n'a pas le droit DNS : enregistrements DNS à faire par Jon dans le tableau de bord.
- Site : vidéo YouTube et lien de don à poser (sections cachées dans `site/index.html`).
- Toujours en attente côté Jon : tests de la `v0.4.0` à `v0.4.4` sous Windows (pastille, son, raccourcis, Tester la clé, export), `.dmg` sur un vrai Mac.
- Wayland GNOME : app lancée sous XWayland (pastille placée, sans vol de focus), floue en échelle fractionnaire sans `xwayland-native-scaling` (décision du 2026-11-16).
- À trancher : séparation des voix limitée à un cœur (sherpa-rs 0.6.8 fixe `num_threads: 1`, il faudrait appeler `sherpa-rs-sys` directement) ; sélecteur de micro dans l'onboarding ; bouton « Retranscrire » avec Mistral ; réunion à un seul micro (tout le monde étiqueté « Moi ») ; fusion de deux locuteurs à la relecture.
- Mineurs laissés de côté : `hasDestination` ignore un chemin fait d'espaces alors que le Rust l'accepte ; en lecture seule, les zones traitées restent surlignées ; « Choisir un dossier » ouvre Réglages sans défiler jusqu'à Destinations ; sourdine du son comptée depuis le micro ; Linux sans `_NET_ACTIVE_WINDOW` ne colle plus ; raccourcis rétablis seulement au `blur` ; F25+ signalées « déjà prises » ; webhook : 30 s sur hôte muet, régénérer écrase le secret sans confirmation ; commentaire de `sound_cues` dans `model.rs` périmé.

## En production
- GitHub : release `v0.4.6`, dépôt public, README à jour sur `main`. Après chaque tag, remplacer les notes générées par des notes en français (format de `v0.4.3`).
- Site : https://mybarnaby.app. Publication à la main : `bunx wrangler@4 pages deploy site --project-name barnaby --branch main`. Tailles et version dans les cartes de L'installer à mettre à jour à chaque release.

## Pièges connus
- Laptop de Jon : `. ~/.cargo/env` ; compiler l'app avec `cargo build --release -p barnaby --features tauri/custom-protocol` (sans la feature, la fenêtre charge `localhost:1420`), après `bun run --cwd ui build`. Un premier build peut échouer sur la copie de `libonnxruntime.so` : `mkdir -p target/release/deps`. Binaire installé à la main dans `~/.local/bin/barnaby` (libs dans `~/.local/lib/barnaby`).
- Portail GNOME : l'app id doit correspondre à un `.desktop` installé (`Barnaby`) ; GNOME garde le déclencheur choisi par id de raccourci.
- VPS sans sudo : sourcer `~/sysroot/env.sh` avant toute compilation ; un seul cargo à la fois (2 CPU) ; jamais `pkill -f vite` ni `cargo fmt` (un `pkill -f` dont le motif figure dans la commande tue aussi le shell).
- Captures de l'app : `bun run dev` (mock), Playwright depuis `~/.npm/_npx/fd3bca3c548369c0/node_modules/playwright` ; le mock ouvre l'onboarding (`onboarding_done: false` dans `ui/src/lib/mock.ts`). Ports 8765 et 8791 pris par d'autres serveurs.
- `.gitignore` ignore `*.wav` : seuls `crates/engine/assets/*.wav` sont suivis (sinon la CI ne trouve pas le son).
- Un push sur `phase-1`, `main` et un tag au même commit lancent trois CI identiques : annuler les deux premières.
- `fr-typography` abîme le README (`**Gras** :`, `?` des titres, attributs HTML) et `site/index.html` (JSON-LD, attribut `allow` de l'iframe) : les remettre après chaque passage.
- macOS : un test qui ouvre le tap attend une permission que personne n'accepte en CI ; signature ad hoc : chaque mise à jour peut faire perdre les permissions.
- sherpa-onnx lié dynamiquement sous Windows et Linux, statiquement sous macOS. Overlay Windows : ne jamais le cacher, seulement le déplacer ; le remonter par `SetWindowPos` (pas `set_always_on_top`, qui finit en `ShowWindow`).
- Pas encore traité : AppImage, `injected_into` vide hors Linux, écho haut-parleur qui fausse « Moi », ~1 Go de RAM par heure de réunion, purge des 7 jours seulement au démarrage et à l'envoi.
