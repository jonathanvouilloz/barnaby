# Reprise : Barnaby
Mise à jour : 2026-10-28

## Où on en est
- Release `v0.4.3` publiée le 2026-10-28 (CI run 40672024862 verte, liens `latest/download` en 200, site à jour). Avant elle, Jon a traité une vraie réunion de 30 min avec Mistral sous Windows : transcription réussie ; validée sans destination, elle n'est allée nulle part (texte resté dans `barnaby.db`, audio supprimé).
- `v0.4.3` contient le plan `docs/plans/meeting-visibility.md` : progression du traitement sur l'accueil + journal par étape, Mistral : progression par morceau et délai 300 s, test de la clé Mistral, réunions validées ouvertes en lecture seule avec « Exporter… », étape « dossier » à l'onboarding, avertissement avant un envoi sans destination.

## Prochaine action
1. Jon teste sous Windows : barre de progression sur une vraie réunion Mistral, « Tester la clé », ouvrir et exporter une réunion envoyée (dont sa réunion de 30 min), onboarding sur une installation neuve.
2. Toujours en attente : tests `v0.4.0` sous Windows et `.dmg` sur un vrai Mac.

## Vérifications (2026-10-28)
- Branche visibilité : `cargo clippy --workspace --all-targets -- -D warnings` propre ; `cargo test -p barnaby-core` 50 ok ; `cargo test -p barnaby-engine diarize -- --ignored` ok ; `bun run check` 0 erreur ; `bun test` 44 ok ; captures du mock relues (accueil en traitement, réunion envoyée, étape dossier, avertissement).
- Non vérifié : avancement réel de la séparation des voix sous Windows, `check_key` contre la vraie API Mistral, fenêtre Mistral avec message d'erreur (jamais capturée).
- Laissés de côté (revue finale) : `hasDestination` ignore un chemin fait d'espaces alors que le Rust l'accepte ; en lecture seule, les zones déjà traitées restent surlignées.

## Vérifications (2026-10-23)
- `cargo clippy --workspace --all-targets -- -D warnings` : propre. `cargo test -p barnaby-core` : 46 ok. `cargo test -p barnaby-engine cue` : 2 ok. `cd ui && bun run check` : 0 erreur ; `bun test` : 41 ok.
- Code Windows (`no_target`, remontée de l'overlay par `SetWindowPos`) : `cargo check --target x86_64-pc-windows-msvc` d'une crate isolée, sans erreur ; jamais exécuté.
- CI run 39983075946 (tag `v0.4.0`) : checks, builds Ubuntu, Windows, macOS et job `release` verts. Liens `releases/latest/download/…` (exe, dmg, deb) : 200.
- Site : `curl https://mybarnaby.app/` et `https://www.mybarnaby.app/` 200, certificat Google Trust Services avec 2 SCT ; contrôles Playwright (pas d'erreur, tout visible après un saut à #questions, mouvement réduit = page fixe, Windows simulé = carte Windows en premier, aucun téléchargement au premier clic).
- Non vérifié : comportement réel sous Windows et macOS ; erreur `ERR_CERTIFICATE_TRANSPARENCY_REQUIRED` vue par des proches de Jon (probablement les premières minutes après l'émission du certificat).

## État non commité
- Rien hors `barnaby.zip` et `.wrangler/` (ignorés). Branche `phase-1`.

## Bloquants et actions du propriétaire
- Le jeton Cloudflare du serveur (`CLOUDFLARE_API_TOKEN`) n'a pas le droit DNS : enregistrements DNS à faire par Jon dans le tableau de bord.
- Site : vidéo YouTube et lien de don à poser (sections cachées dans `site/index.html`) ; Jon prévoit une ou deux vidéos d'installation.
- Séparation des voix limitée à un cœur (sherpa-rs 0.6.8 fixe `num_threads: 1`) : plus lente sur une longue réunion.
- À trancher : sélecteur de micro dans l'onboarding ; bouton « Retranscrire » avec Mistral ; réunion à un seul micro (tout le monde étiqueté « Moi »).
- Mineurs laissés de côté (revue finale) : sourdine du son comptée depuis le micro et pas depuis la lecture ; Linux sans `_NET_ACTIVE_WINDOW` (dwm…) ne colle plus ; raccourcis rétablis seulement au `blur` ; F25+ signalées « déjà prises » ; webhook : 30 s sur hôte muet, régénérer écrase le secret sans confirmation ; commentaire de `sound_cues` dans `model.rs` qui parle encore de l'arrêt.

## En production
- GitHub : release `v0.4.0`, dépôt public.
- Site : https://mybarnaby.app. Publication à la main : `bunx wrangler@4 pages deploy site --project-name barnaby --branch main`. Tailles et version dans les cartes de L'installer à mettre à jour à chaque release.

## Pièges connus
- VPS sans sudo : sourcer `~/sysroot/env.sh` avant toute compilation ; un seul cargo à la fois (2 CPU) ; jamais `pkill -f vite` ni `cargo fmt`.
- Captures de l'app : `bun run dev` (mock), Playwright depuis `~/.npm/_npx/fd3bca3c548369c0/node_modules/playwright` ; le mock ouvre l'onboarding (`onboarding_done: false` dans `ui/src/lib/mock.ts`). Le port 8765 est pris par une autre session.
- `.gitignore` ignore `*.wav` : seuls `crates/engine/assets/*.wav` sont suivis (sinon la CI ne trouve pas le son).
- Un push sur `phase-1`, `main` et un tag au même commit lancent trois CI identiques : annuler les deux premières.
- `fr-typography` abîme le JSON-LD et l'attribut `allow` de l'iframe dans `site/index.html` : les remettre après chaque passage.
- macOS : un test qui ouvre le tap attend une permission que personne n'accepte en CI ; signature ad hoc : chaque mise à jour peut faire perdre les permissions.
- sherpa-onnx lié dynamiquement sous Windows et Linux, statiquement sous macOS. Overlay Windows : ne jamais le cacher, seulement le déplacer ; le remonter par `SetWindowPos` (pas `set_always_on_top`, qui finit en `ShowWindow`).
- Pas encore traité : AppImage, `injected_into` vide hors Linux, écho haut-parleur qui fausse « Moi », ~1 Go de RAM par heure de réunion, purge des 7 jours seulement au démarrage et à l'envoi.
