# Reprise : Barnaby
Mise à jour : 2026-10-23

## Où on en est
- Release `v0.4.0` publiée (Windows `.exe`/`.msi`, Linux `.deb`, macOS `.dmg`), dernière version sur GitHub. Contenu : plan `docs/plans/dictation-polish.md` (son de démarrage, overlay remonté et transparent aux clics, saisie de raccourci réparée + touche seule + avertissements, bouton Parcourir…, webhook Tester + secret généré, texte gardé au presse-papiers + notification sans fenêtre active). Tâches 1 à 5 faites, revue indépendante faite (2 points importants corrigés) ; reste l'étape 3 de la tâche 6 (test de Jon sous Windows).
- Site en ligne sur https://mybarnaby.app (Cloudflare Pages, projet `barnaby`) : GSAP, questions qui se déplient, section « Qui l'a fait », téléchargement en deux temps (cartes par système), liens externes dans un nouvel onglet.
- `main` = `phase-1`.

## Prochaine action
1. Jon installe `v0.4.0` sous Windows et vérifie : pastille devant une fenêtre « toujours devant » sans voler le focus ; son au démarrage, absent du texte ; Ctrl+P ressaisi dans Réglages ; Parcourir… ; Tester ; dictée sur le bureau puis Ctrl+V.
2. Puis le `.dmg` sur un Mac (rien n'a tourné sur un vrai Mac).

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
