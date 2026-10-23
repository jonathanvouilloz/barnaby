# Reprise — Barnaby
Mise à jour : 2026-10-22

## Où on en est
- `phase-1` fusionnée dans `main` (PR #1). Release `v0.2.0` publiée avec `Barnaby-windows-setup.exe`, `Barnaby-windows.msi`, `Barnaby-linux-amd64.deb`, `Barnaby-macos-arm64.dmg`. Les builds du tag sont passés (run 39829286125), mais GitHub n'a pas lancé le job `release` (quota Actions du compte) : release créée à la main avec `gh release create` à partir de ces artefacts.
- Version macOS faite (plan `docs/plans/macos.md`, décision du 2026-10-22) : tap Core Audio global pour le son système, `.dmg` signé ad hoc, macOS 14.2+ sur puce Apple. Revue de code indépendante faite, corrections appliquées (collage par position de touche, arrêt hors du fil principal, droit micro, raccourci ⌘⇧Espace).
- Site de présentation fait dans `site/` (cadrage `docs/project/site.md`, section Site de `DESIGN.md`), validé par Jon ; pas encore en ligne.

## Prochaine action
1. Jon teste le `.dmg` sur un Mac : installation, dictée (⌘⇧Espace), réunion avec du son système (visio), permissions.
2. Liste de l'app faite et publiée en `v0.3.0` (run 39875559898, job `release` vert, quatre fichiers) (logo, barre de titre, badge « Recommandé », réglage du son renommé) : à tester dans un vrai build (barre de titre Windows et Linux : déplacer, redimensionner, double-clic), puis le test du son sous Windows.

## Vérifications (2026-10-22)
- CI run 39829286125 (tag `v0.2.0`) : checks, build Ubuntu, Windows et macOS-14 verts ; artefacts `.deb`, `.exe`/`.msi`, `.dmg` (14 Mo) ; job `release` non démarré (facturation).
- `gh api …/releases/latest` : `v0.2.0` avec les quatre fichiers.
- Local : `cargo clippy --workspace --all-targets -- -D warnings` propre ; `cargo test -p barnaby-core` 44 ok ; `cd ui && bun test` 40 ok, `bun run check` 0 erreur.
- Module macOS vérifié à part : `cargo check --target aarch64-apple-darwin` sans erreur (crate isolée, pas de SDK Apple ici).
- Dépôt public depuis le 2026-10-22 : liens `releases/latest/download/…` (dmg, exe, deb) → 200 sans connexion ; Actions relancées (run 39871635953).
- Non vérifié : rien n'a tourné sur un vrai Mac (tap, agrégat privé visible par cpal, overlay transparent, collage, permissions) ; contenu du `.dmg` non inspecté.

## État non commité
- Rien hors `barnaby.zip` (dépôt de Jon, non versionné). `main` et `phase-1` au même commit.

## Bloquants et actions du propriétaire
- Site : adresse finale (jonlabs.ch/barnaby ou un domaine) pour la balise canonique, `og:url` et `og:image` ; vidéo YouTube et lien de don à poser (sections cachées dans `site/index.html`).
- À trancher : sélecteur de micro dans l'onboarding ; bouton « Retranscrire » avec Mistral.
- Réunion à un seul micro (analyse du 2026-10-22) : sans son système, tout le monde est étiqueté « Moi », ce qui fausse les tâches du résumé ; `whisper-small` par défaut est faible en salle. Correctifs proposés, pas encore décidés.

## En production
- GitHub : dernière release `v0.3.0` (Windows, Linux, macOS), dépôt public.
- Site : non déployé (prévu sur Cloudflare).

## Pièges connus
- VPS sans sudo : sourcer `~/sysroot/env.sh` avant toute compilation ; un seul cargo à la fois (2 CPU) ; jamais `pkill -f vite` ni `cargo fmt`.
- Captures : `bun run dev` (mock), Playwright depuis `~/.npm/_npx/fd3bca3c548369c0/node_modules/playwright`. Le port 8765 est pris par une autre session.
- `fr-typography` abîme le JSON-LD et l'attribut `allow` de l'iframe dans `site/index.html` : les remettre après chaque passage.
- macOS : un test qui ouvre le tap attend une permission que personne n'accepte en CI (511 s puis échec) ; exclure les vraies sorties dans les tests. Signature ad hoc : chaque mise à jour peut faire perdre les permissions accordées.
- sherpa-onnx lié dynamiquement sous Windows et Linux (rpath `$ORIGIN/../lib/barnaby`), statiquement sous macOS. Overlay Windows : ne jamais le cacher, seulement le déplacer.
- Pas encore traité : AppImage, `injected_into` vide hors Linux, écho haut-parleur qui fausse « Moi », ~1 Go de RAM par heure de réunion, purge des 7 jours seulement au démarrage et à l'envoi, téléchargement de modèle bloqué si le serveur cesse d'envoyer.
