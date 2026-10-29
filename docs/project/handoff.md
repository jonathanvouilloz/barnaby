# Reprise : Barnaby
Mise à jour : 2026-10-29

## Où on en est
- Release `v0.4.3` publiée le 2026-10-28 (plan `docs/plans/meeting-visibility.md` terminé). Décisions dans `decisions.md`.
- 2026-10-29 : README refait (lien mybarnaby.app, téléchargements, captures, premiers pas, questions), notes en français écrites pour les releases `v0.1.0` à `v0.4.2` (elles étaient vides), site du dépôt GitHub = https://mybarnaby.app. Test `mix_follows_the_wall_clock` borné par le temps réellement écoulé (échouait sur le runner macOS lent).
- Première vraie réunion Mistral de Jon (30 min, Windows) : texte dans `barnaby.db`, récupérable par « Exporter… » en `v0.4.3`.
- `main` = `phase-1` = `372a46e` (+ ce commit de docs sur `phase-1`).

## Prochaine action
1. Jon lance `cargo test -p barnaby-engine mix_follows_the_wall_clock` sous Windows, puis installe `v0.4.3` et vérifie : barre de progression sur une vraie réunion Mistral, « Tester la clé », ouvrir et exporter sa réunion de 30 min, onboarding sur une installation neuve (étape dossier).
2. Toujours en attente : tests de la `v0.4.0` sous Windows (pastille, son, raccourcis, Parcourir…, Tester) et `.dmg` sur un vrai Mac.

## Vérifications (2026-10-29)
- `cargo test -p barnaby-engine mix_follows_the_wall_clock` : 1 ok ; `cargo clippy -p barnaby-engine --all-targets -- -D warnings` : propre.
- CI run 40856572678 (`main`, `372a46e`) : checks, macOS, Linux, Windows verts.
- `releases/latest/download/…` (exe, msi, dmg, deb) : 200 ; `curl https://mybarnaby.app/` affiche « Version 0.4.3 ». README rendu par `gh api markdown` : 7 images, ancres ok ; liens externes : 200.
- Vérifiés le 2026-10-28 et inchangés : `cargo test -p barnaby-core` 51 ok, `bun run check` 0 erreur, `bun test` 45 ok.
- Doc-reviewer sur `c811125..372a46e` : 0 faux après correction (« seule option payante » oubliait le résumé OpenRouter, corrigé). Invérifiables : libellés des avertissements d'installation Windows et Mac, « Mistral plus rapide sur un petit ordinateur » (aucune mesure). Le site dit encore « La seule chose payante » : à corriger.
- Non vérifié : avancement réel de la séparation des voix sous Windows ; `check_key` contre la vraie API Mistral ; comportement réel sous macOS.

## État non commité
- Rien hors `barnaby.zip` et `.wrangler/` (ignorés). Branche `phase-1`.

## Bloquants et actions du propriétaire
- Le jeton Cloudflare du serveur (`CLOUDFLARE_API_TOKEN`) n'a pas le droit DNS : enregistrements DNS à faire par Jon dans le tableau de bord.
- Site : vidéo YouTube et lien de don à poser (sections cachées dans `site/index.html`) ; Jon prévoit une ou deux vidéos d'installation.
- À trancher : sélecteur de micro dans l'onboarding ; bouton « Retranscrire » avec Mistral ; réunion à un seul micro (tout le monde étiqueté « Moi ») ; séparation des voix limitée à un cœur (sherpa-rs 0.6.8 fixe `num_threads: 1`), lente sur une longue réunion.
- Mineurs laissés de côté : `hasDestination` ignore un chemin fait d'espaces alors que le Rust l'accepte ; en lecture seule, les zones traitées restent surlignées ; « Choisir un dossier » ouvre Réglages sans défiler jusqu'à Destinations ; sourdine du son comptée depuis le micro ; Linux sans `_NET_ACTIVE_WINDOW` ne colle plus ; raccourcis rétablis seulement au `blur` ; F25+ signalées « déjà prises » ; webhook : 30 s sur hôte muet, régénérer écrase le secret sans confirmation ; commentaire de `sound_cues` dans `model.rs` périmé.

## En production
- GitHub : release `v0.4.3`, dépôt public, README à jour sur `main`. Après chaque tag, remplacer les notes générées par des notes en français (format de `v0.4.3`).
- Site : https://mybarnaby.app. Publication à la main : `bunx wrangler@4 pages deploy site --project-name barnaby --branch main`. Tailles et version dans les cartes de L'installer à mettre à jour à chaque release.

## Pièges connus
- VPS sans sudo : sourcer `~/sysroot/env.sh` avant toute compilation ; un seul cargo à la fois (2 CPU) ; jamais `pkill -f vite` ni `cargo fmt` (un `pkill -f` dont le motif figure dans la commande tue aussi le shell).
- Captures de l'app : `bun run dev` (mock), Playwright depuis `~/.npm/_npx/fd3bca3c548369c0/node_modules/playwright` ; le mock ouvre l'onboarding (`onboarding_done: false` dans `ui/src/lib/mock.ts`). Ports 8765 et 8791 pris par d'autres serveurs.
- `.gitignore` ignore `*.wav` : seuls `crates/engine/assets/*.wav` sont suivis (sinon la CI ne trouve pas le son).
- Un push sur `phase-1`, `main` et un tag au même commit lancent trois CI identiques : annuler les deux premières.
- `fr-typography` sur le README colle `**Gras** :` en `**Gras**:`, décale le `?` des titres et touche les attributs HTML (`prefers-color-scheme: dark`, `?label=`) : les remettre après.
- `fr-typography` abîme le JSON-LD et l'attribut `allow` de l'iframe dans `site/index.html` : les remettre après chaque passage.
- macOS : un test qui ouvre le tap attend une permission que personne n'accepte en CI ; signature ad hoc : chaque mise à jour peut faire perdre les permissions.
- sherpa-onnx lié dynamiquement sous Windows et Linux, statiquement sous macOS. Overlay Windows : ne jamais le cacher, seulement le déplacer ; le remonter par `SetWindowPos` (pas `set_always_on_top`, qui finit en `ShowWindow`).
- Pas encore traité : AppImage, `injected_into` vide hors Linux, écho haut-parleur qui fausse « Moi », ~1 Go de RAM par heure de réunion, purge des 7 jours seulement au démarrage et à l'envoi.
