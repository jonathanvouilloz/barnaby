# Reprise : Barnaby
Mise à jour : 2026-10-30

## Où on en est
- Dernière release : `v0.4.4` (2026-10-30), notes en français, site à jour (« Version 0.4.4 », tailles inchangées). `main` = `phase-1`.
- Contenu : nombre de voix = participants saisis + 1 (dès une minute), transcription et séparation des voix en même temps, Mistral 3 blocs à la fois, prénom de l'utilisateur (onboarding et Réglages › Réunion) et ligne « Moi » fixe dans « Nouvelle réunion ».

## Prochaine action
1. Jon installe `v0.4.4`, met son prénom dans Réglages › Réunion, relance sa réunion à 5 (sans s'ajouter) et note : nombre de locuteurs, durée totale (le journal donne la durée de chaque étape et de chaque bloc Mistral).
2. Ensuite, au choix de Jon : séparation des voix sur plusieurs cœurs (voir Bloquants), test de la séparation des voix de Voxtral 3 sur une réunion entière.

## Vérifications (2026-10-30)
- `cargo test -p barnaby-core` : 55 ok (nouveau : `user_name_from_settings_names_me`).
- `cd ui && bun test` : 45 ok (`speaker names` étendu) ; `bun run check` : 0 erreur.
- Captures mock (onboarding, Nouvelle réunion, Réglages ; clair, sombre, 1440 et 900 px) dans `design/shots/` : conformes à `refs.md`.
- CI run 41092435001 (tag `v0.4.4`) : checks, Linux, Windows, macOS, release verts. `curl https://mybarnaby.app` : « Version 0.4.4 ».
- Non vérifié : gain de temps réel, nombre de locuteurs sur une vraie réunion, Mistral avec 3 requêtes simultanées (risque de `mistral_quota`), clippy du workspace sur le VPS (fait par la CI).

## État non commité
- Rien hors `barnaby.zip` et `.wrangler/` (ignorés). Branche `phase-1`.

## Bloquants et actions du propriétaire
- Le jeton Cloudflare du serveur (`CLOUDFLARE_API_TOKEN`) n'a pas le droit DNS : enregistrements DNS à faire par Jon dans le tableau de bord.
- Site : vidéo YouTube et lien de don à poser (sections cachées dans `site/index.html`).
- Toujours en attente côté Jon : tests de la `v0.4.0` à `v0.4.4` sous Windows (pastille, son, raccourcis, Tester la clé, export), `.dmg` sur un vrai Mac.
- À trancher : séparation des voix limitée à un cœur (sherpa-rs 0.6.8 fixe `num_threads: 1`, il faudrait appeler `sherpa-rs-sys` directement) ; sélecteur de micro dans l'onboarding ; bouton « Retranscrire » avec Mistral ; réunion à un seul micro (tout le monde étiqueté « Moi ») ; fusion de deux locuteurs à la relecture.
- Mineurs laissés de côté : `hasDestination` ignore un chemin fait d'espaces alors que le Rust l'accepte ; en lecture seule, les zones traitées restent surlignées ; « Choisir un dossier » ouvre Réglages sans défiler jusqu'à Destinations ; sourdine du son comptée depuis le micro ; Linux sans `_NET_ACTIVE_WINDOW` ne colle plus ; raccourcis rétablis seulement au `blur` ; F25+ signalées « déjà prises » ; webhook : 30 s sur hôte muet, régénérer écrase le secret sans confirmation ; commentaire de `sound_cues` dans `model.rs` périmé.

## En production
- GitHub : release `v0.4.4`, dépôt public, README à jour sur `main`. Après chaque tag, remplacer les notes générées par des notes en français (format de `v0.4.3`).
- Site : https://mybarnaby.app. Publication à la main : `bunx wrangler@4 pages deploy site --project-name barnaby --branch main`. Tailles et version dans les cartes de L'installer à mettre à jour à chaque release.

## Pièges connus
- VPS sans sudo : sourcer `~/sysroot/env.sh` avant toute compilation ; un seul cargo à la fois (2 CPU) ; jamais `pkill -f vite` ni `cargo fmt` (un `pkill -f` dont le motif figure dans la commande tue aussi le shell).
- Captures de l'app : `bun run dev` (mock), Playwright depuis `~/.npm/_npx/fd3bca3c548369c0/node_modules/playwright` ; le mock ouvre l'onboarding (`onboarding_done: false` dans `ui/src/lib/mock.ts`). Ports 8765 et 8791 pris par d'autres serveurs.
- `.gitignore` ignore `*.wav` : seuls `crates/engine/assets/*.wav` sont suivis (sinon la CI ne trouve pas le son).
- Un push sur `phase-1`, `main` et un tag au même commit lancent trois CI identiques : annuler les deux premières.
- `fr-typography` abîme le README (`**Gras** :`, `?` des titres, attributs HTML) et `site/index.html` (JSON-LD, attribut `allow` de l'iframe) : les remettre après chaque passage.
- macOS : un test qui ouvre le tap attend une permission que personne n'accepte en CI ; signature ad hoc : chaque mise à jour peut faire perdre les permissions.
- sherpa-onnx lié dynamiquement sous Windows et Linux, statiquement sous macOS. Overlay Windows : ne jamais le cacher, seulement le déplacer ; le remonter par `SetWindowPos` (pas `set_always_on_top`, qui finit en `ShowWindow`).
- Pas encore traité : AppImage, `injected_into` vide hors Linux, écho haut-parleur qui fausse « Moi », ~1 Go de RAM par heure de réunion, purge des 7 jours seulement au démarrage et à l'envoi.
