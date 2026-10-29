# Reprise : Barnaby
Mise à jour : 2026-10-29

## Où on en est
- Dernière release : `v0.4.3` (2026-10-28). README et notes de release en français à jour.
- Retour de Jon sur ses vraies réunions Mistral en visio : 15 min de traitement pour 30 à 50 min de réunion, et une quinzaine de locuteurs pour 5 personnes. Corrigé dans `6ffbc75` sur `phase-1`, pas encore publié (décisions du 2026-10-29) :
  - la séparation des voix cherche « participants saisis + 1 » voix dès une minute d'audio, seuil 0,7 sans liste ;
  - transcription et séparation des voix en même temps, Mistral 3 blocs à la fois (délai 900 s par requête).
- `main` = `fcc8db4` ; `phase-1` a en plus le handoff précédent, `6ffbc75` et ce commit de docs.

## Prochaine action
1. Jon installe l'artefact de la CI run 40970261598 (https://github.com/jonathanvouilloz/barnaby/actions/runs/40970261598), se retire de la liste des participants s'il s'y était mis, relance le traitement de sa réunion à 5 et note : nombre de locuteurs, durée totale (le journal donne la durée de chaque étape et de chaque bloc Mistral).
2. Si c'est bon : tag `v0.4.4` avec son accord, notes en français.
3. Ensuite, au choix de Jon : séparation des voix sur plusieurs cœurs (voir Bloquants), test de la séparation des voix de Voxtral 3 sur une réunion entière.

## Vérifications (2026-10-29)
- `cargo test -p barnaby-core` : 54 ok (nouveaux : `blocks_keep_their_order_and_offsets`, `transcription_and_diarization_run_together`, `speaker_count_adds_the_user_and_needs_a_minute`).
- `cargo clippy -p barnaby-core --all-targets -- -D warnings` : propre. Clippy du workspace impossible sur le VPS (alsa, glib manquants).
- `cd ui && bun test` : 45 ok ; `bun run check` : 0 erreur.
- CI run 40970261598 (`phase-1`, `6ffbc75`) : checks, Linux, Windows, macOS verts (clippy workspace et tests engine compris).
- Non vérifié : gain de temps réel, nombre de locuteurs sur une vraie réunion, comportement de Mistral avec 3 requêtes simultanées (risque de `mistral_quota`).

## État non commité
- Rien hors `barnaby.zip` et `.wrangler/` (ignorés). Branche `phase-1`.

## Bloquants et actions du propriétaire
- Le jeton Cloudflare du serveur (`CLOUDFLARE_API_TOKEN`) n'a pas le droit DNS : enregistrements DNS à faire par Jon dans le tableau de bord.
- Site : vidéo YouTube et lien de don à poser (sections cachées dans `site/index.html`).
- Toujours en attente côté Jon : tests de la `v0.4.0` et `v0.4.3` sous Windows (pastille, son, raccourcis, Tester la clé, export), `.dmg` sur un vrai Mac.
- À trancher : séparation des voix limitée à un cœur (sherpa-rs 0.6.8 fixe `num_threads: 1`, il faudrait appeler `sherpa-rs-sys` directement) ; sélecteur de micro dans l'onboarding ; bouton « Retranscrire » avec Mistral ; réunion à un seul micro (tout le monde étiqueté « Moi ») ; fusion de deux locuteurs à la relecture.
- Mineurs laissés de côté : `hasDestination` ignore un chemin fait d'espaces alors que le Rust l'accepte ; en lecture seule, les zones traitées restent surlignées ; « Choisir un dossier » ouvre Réglages sans défiler jusqu'à Destinations ; sourdine du son comptée depuis le micro ; Linux sans `_NET_ACTIVE_WINDOW` ne colle plus ; raccourcis rétablis seulement au `blur` ; F25+ signalées « déjà prises » ; webhook : 30 s sur hôte muet, régénérer écrase le secret sans confirmation ; commentaire de `sound_cues` dans `model.rs` périmé.

## En production
- GitHub : release `v0.4.3`, dépôt public, README à jour sur `main`. Après chaque tag, remplacer les notes générées par des notes en français (format de `v0.4.3`).
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
