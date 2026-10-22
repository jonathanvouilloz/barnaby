# Refonte « Things, version chouette » et onboarding — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** nouvelle apparence de toute l'interface (Valley Sans, palette de la chouette, étincelle ✦) et onboarding au premier lancement.

**Architecture:** tokens et composants de base dans `ui/src/app.css`, coquille dans `App.svelte`, un composant `Spark.svelte` pour la signature, `Owl.svelte` redessiné. Chaque écran est restylé sans changer son comportement. L'onboarding est un écran plein (`ui/src/onboarding/`) qui remplace `ModelSetup.svelte`, piloté par un réglage `onboarding_done` et deux commandes de test du micro.

**Tech Stack:** Svelte 5, TypeScript, Bun, Vite ; Rust (Tauri 2) pour le réglage et le test du micro.

**Spec:** `docs/project/redesign-onboarding.md`. Direction : `.impeccable/surfaces/ui-src-app-svelte.md`. Références : `design/references/refs.md`.

## Global Constraints

- Police : `@fontsource-variable/valley-sans` 5.3.1 (dépendance de `ui/`), importée dans `ui/src/main.ts` ; `--font: "Valley Sans Variable", system-ui, -apple-system, "Segoe UI Variable Text", "Segoe UI", sans-serif`. Aucun chargement réseau.
- Tokens clair : `--bg #fdfdfd`, `--surface #ffffff`, `--sidebar rgb(248 235 220 / .82)`, `--text #0f0c2f`, `--text-2 #6f5b52`, `--border rgb(15 12 47 / .09)`, `--accent #0170ec`, `--on-accent #fdfdfd`, `--rec #fcaa4f` (fond, texte `--text` dessus), `--danger #b3261e`, `--warn #9a5b00`, `--ok #1f7a3a`, `--fill rgb(15 12 47 / .05)`, `--fill-strong rgb(15 12 47 / .09)`, `--spark #0170ec`.
- Tokens sombre : `--bg #0f0c2f`, `--surface #18143d`, `--sidebar rgb(26 22 64 / .78)`, `--text #f8ebdc`, `--text-2 #b9a596`, `--border rgb(248 235 220 / .12)`, `--accent #4d9bff`, `--on-accent #0f0c2f`, `--rec #fcaa4f`, `--danger #ff8a80`, `--warn #ffc069`, `--ok #5bd17f`, `--fill rgb(248 235 220 / .06)`, `--fill-strong rgb(248 235 220 / .11)`, `--spark #4d9bff`.
- Chouette (les deux thèmes) : `--owl-line #0f0c2f`, `--owl-body #b9a596`, `--owl-shade #9f887d`, `--owl-face #f8ebdc`, `--owl-belly #decfc5`, `--owl-beak #fcaa4f`, `--owl-eye #0f0c2f`.
- Échelle : titre d'écran 28px/700, titre de section 15px/650, corps 14px/400 interligne 1.5, secondaire 12.5px. Rayons 8px (contrôles), 12px (cartes), 16px (feuilles). Espacements multiples de 4px.
- Mouvement : 150 à 240 ms, `--ease: cubic-bezier(.2, .7, .3, 1)` ; tout est coupé sous `prefers-reduced-motion` (règle globale existante).
- Interdits (craft floor d'impeccable et DESIGN actuel) : pas d'eyebrow, pas de dégradé décoratif, pas de verre sur le contenu, pas de bordure latérale colorée en accent, pas de carte dans une carte.
- Comportement inchangé : routes, appels `api.*`, événements, raccourcis, accessibilité (focus visible, `aria-*`, `aria-live`). Interface française au tutoiement.
- Chaînes visibles uniquement via `ui/src/lib/i18n.ts` (fr et en). Typographie française par skill `fr-typography`, en relisant ensuite les octets : le script transforme « › » en « » » et n'ajoute pas l'espace insécable avant « $ » ; ces deux cas se corrigent à la main.
- Vérifications : `cd ui && bun run check && bun test` ; Rust : `. ~/sysroot/env.sh`, `-j2`, `cargo clippy --workspace --all-targets -j2 -- -D warnings`, `LD_LIBRARY_PATH=$HOME/sysroot/root/usr/lib/x86_64-linux-gnu cargo test --workspace -j2`. Ne jamais lancer `cargo fmt` ni `pkill -f vite`.

## Review Focus

1. Mode sombre : chaque écran lisible (texte, bordures, états) ; aucun blanc codé en dur (test dans la tâche 2 : grep des couleurs littérales hors `app.css`).
2. Installation existante avec tous les modèles et `onboarding_done` faux : l'onboarding ne s'affiche pas et le réglage passe à vrai (test dans la tâche 6).
3. Accès au micro refusé ou aucun micro pendant l'onboarding : message clair, l'étape reste passable (géré dans la tâche 6 ; erreur `capture.*` de `start_mic_test`).
4. Téléchargement de modèle en échec au milieu : l'erreur s'affiche, « Réessayer » reprend sans retélécharger ce qui est fini (test dans la tâche 6).
5. Fenêtre étroite (900 px de large, minimum Tauri) : barre latérale et écrans sans débordement horizontal (vérifié aux captures, tâche 7).

---

### Task 1: Réglage `onboarding_done` et test du micro (Rust)

**Files:**
- Modify: `crates/core/src/model.rs` (Settings), `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/dictation.rs` (rendre `level` `pub(crate)`)

**Interfaces:**
- Produces: `Settings.onboarding_done: bool` (`#[serde(default)]`, défaut `false`) ; commandes `start_mic_test() -> CmdResult<()>` (erreur `capture.no_mic` ou `capture.permission_denied`) et `stop_mic_test()` ; événement `mic-level` (f32 0..1, même échelle que `dictation-level`, 20 par seconde) ; côté UI `api.startMicTest()`, `api.stopMicTest()`, `events.micLevel(cb)`.

- [x] **Step 1:** Test dans `model.rs` : un JSON de réglages sans `onboarding_done` se lit avec `false` ; les défauts valent `false`.
- [x] **Step 2:** Échec attendu, puis ajouter le champ (Rust et `api.ts`, même commit).
- [x] **Step 3:** `start_mic_test` ouvre un `DictationRecorder` dans un fil qui émet `mic-level` toutes les 50 ms jusqu'à `stop_mic_test` (un `Sender` d'arrêt dans `App`, sur le modèle de `dictation::Session`). Un second `start` pendant un test en cours ne fait rien. Le micro n'est jamais enregistré.
- [x] **Step 4:** `mock.ts` : les deux commandes, et un `mic-level` simulé (onde lente) tant que le test tourne.
- [x] **Step 5:** Vérifications Rust et UI, commit `add onboarding flag and mic test`.

### Task 2: Fondations visuelles

**Files:**
- Modify: `ui/package.json`, `ui/bun.lock`, `ui/src/main.ts`, `ui/src/app.css`, `ui/src/App.svelte`, `ui/src/lib/Owl.svelte`, `ui/src/lib/Sheet.svelte`, `ui/src/lib/StateBadge.svelte`
- Create: `ui/src/lib/Spark.svelte`, `ui/src/lib/Icon.svelte`
- Test: `ui/src/lib/lib.test.ts`

**Interfaces:**
- Produces: tous les tokens des Global Constraints dans `app.css` ; classes `.btn` (`primary`, `ghost`, `danger`, `small`, `icon`), `.field`, `.card`, `.screen`, `.screen-head`, `.screen-body`, `.secondary`, `.empty` restylées en gardant leurs noms ; `<Spark size={number} active={boolean} label?={string} />` (✦ en SVG, couleur `--spark` ; quand `active` passe de vrai à faux, une animation de 240 ms : rotation de 90° et éclat, puis disparition) ; `<Icon name="home|review|settings|record|mic|spark|check|chevron" />` (SVG 16px, trait 1.75, `currentColor`) ; `Owl.svelte` garde son API (`state`, `size`).

- [x] **Step 1:** Test : aucun fichier `.svelte` sous `ui/src` ne contient de couleur littérale (`#rgb`, `#rrggbb`, `rgb(`) hors `var(...)` et hors `Owl.svelte` qui lit ses propres tokens. Échoue aujourd'hui s'il en existe ; sinon il protège la suite.
- [x] **Step 2:** `bun add @fontsource-variable/valley-sans@5.3.1`, import dans `main.ts`, tokens clair et sombre, styles de base.
- [x] **Step 3:** Coquille : barre latérale translucide (`backdrop-filter`) aux tons `--sidebar`, nom « Barnaby » avec la petite étincelle, entrées avec icône colorée (Accueil `--accent`, Relecture `--spark` avec compteur ✦ à droite, Réglages `--text-2`), bouton « Nouvelle réunion » en bas (pastille `--rec` à gauche). La pastille d'état actuelle reste (enregistrement : point `--rec` qui pulse). La chouette ne figure plus dans la barre latérale (elle vit aux moments clés).
- [x] **Step 4:** `Owl.svelte` redessiné d'après `design/references/owl-sheet.png` : tête ronde à deux aigrettes, disque facial crème, grands yeux encre avec reflet, bec orange, ventre clair, pattes orange, trait encre ; une expression par état (`idle` neutre, `listening` yeux ouverts et étincelle, `thinking` yeux vers le haut, `review` tête penchée, `success` yeux plissés de joie, `error` bouche ronde). Animations douces (clignement, petite oscillation), coupées sous réduction des mouvements.
- [x] **Step 5:** `bun run check && bun test`, commit `new visual foundation: tokens, font, shell, owl, spark`.

### Task 3: Écrans du quotidien

**Files:**
- Modify: `ui/src/screens/Home.svelte`, `NewMeeting.svelte`, `Recording.svelte`, `ReviewQueue.svelte`, `ui/src/Overlay.svelte`

**Interfaces:**
- Consumes: tokens, classes, `Spark`, `Icon`, `Owl` (tâche 2).

- [x] **Step 1:** Accueil : titre 28px, liste des réunions groupées par jour comme dans Things (en-tête de jour discret, lignes sans bordure, survol `--fill`), état en clair à droite, ✦ sur les réunions à relire ; dictées récentes en dessous ; écran vide avec la chouette `idle` et une phrase. Action principale « Nouvelle réunion » en haut à droite.
- [x] **Step 2:** Nouvelle réunion et Enregistrement : formulaire aéré ; pendant l'enregistrement, niveaux micro et système en barres arrondies, minuteur en chiffres tabulaires, bouton d'arrêt `--rec`.
- [x] **Step 3:** File de relecture : liste avec ✦ et nombre de passages ; vide : chouette `success` et « Rien à relire ».
- [x] **Step 4:** Overlay de dictée : mêmes couleurs et police, forme inchangée.
- [x] **Step 5:** Vérifications UI, commit `restyle home, meeting and queue screens`.

### Task 4: Relecture et réglages

**Files:**
- Modify: `ui/src/screens/Review.svelte`, `ReviewZone.svelte`, `Settings.svelte`

- [x] **Step 1:** Relecture : transcript en colonne de lecture confortable (68 caractères environ), passages douteux surlignés `--fill-strong` avec ✦ en marge, passage en cours entouré `--spark` ; panneau des questions à droite, une question à la fois, actions claires ; quand un passage est résolu, son ✦ joue l'animation de `Spark`.
- [x] **Step 2:** Réglages : sections en groupes façon réglages macOS (titre de section, lignes label/contrôle alignées, séparateurs fins), pas de carte dans une carte ; bouton « Revoir l'accueil » qui remet `onboarding_done` à faux et ouvre l'onboarding (route `onboarding`, ajoutée en tâche 6 : dans cette tâche, le bouton appelle `go({ name: 'onboarding' })` et la route est déclarée dans `Route`).
- [x] **Step 3:** Vérifications UI, commit `restyle review and settings`.

### Task 5: Tutoiement

**Files:**
- Modify: `ui/src/lib/i18n.ts`, `crates/core` ou `src-tauri` seulement si un texte visible y vit (notifications de `dictation.rs` et `meeting.rs`).

- [x] **Step 1:** Toutes les chaînes françaises passent au tutoiement, ton complice et simple (phrases complètes, pas de jargon, pas de majuscule à chaque mot). Anglais : même ton, direct et chaleureux. Les clés ne changent pas.
- [x] **Step 2:** Notifications système en Rust (`dictation.rs` `alert`, `meeting.rs` `notify`) : même passage.
- [x] **Step 3:** Skill `fr-typography`, puis relecture des octets (› et $, voir Global Constraints).
- [x] **Step 4:** Vérifications UI et Rust, commit `switch French copy to tu`.

### Task 6: Onboarding

**Files:**
- Create: `ui/src/onboarding/Onboarding.svelte`, `ui/src/onboarding/steps.ts`, et un composant par étape au besoin (`Welcome`, `Mic`, `Models`, `DictationTest`, `Tour`, `Done`)
- Modify: `ui/src/App.svelte`, `ui/src/lib/state.svelte.ts` (route `onboarding`), `ui/src/lib/i18n.ts`, `ui/src/lib/lib.test.ts`
- Delete: `ui/src/lib/ModelSetup.svelte` (et ses chaînes `setup.*` inutilisées)

**Interfaces:**
- Consumes: `api.startMicTest/stopMicTest`, `events.micLevel`, `Settings.onboarding_done` (tâche 1) ; `missingRequired` (`models.ts`) ; `MISTRAL_MODEL`, `modeFor`, la feuille Mistral de `Settings.svelte` (extraire en composant partagé `ui/src/lib/MistralSheet.svelte` si les deux écrans s'en servent) ; `ShortcutInput` ; `Owl`, `Spark`.
- Produces (`steps.ts`, pur) : `export type Step = 'welcome' | 'mic' | 'models' | 'dictation' | 'tour' | 'done'` ; `export const STEPS: Step[]` dans cet ordre ; `export function shouldOnboard(settings: Settings, missing: ModelInfo[]): 'show' | 'skip' | 'mark_done'` (`show` si pas fait et des modèles manquent, `mark_done` si pas fait et rien ne manque, `skip` si fait) ; `export function modelsFor(choice: 'local' | 'cloud', models: ModelInfo[], settings: Settings): ModelInfo[]` (`missingRequired` avec `meeting.model` remplacé par `MISTRAL_MODEL` pour `cloud`).

- [x] **Step 1: tests** dans `lib.test.ts` : `shouldOnboard` sur les trois cas ; `modelsFor('cloud', …)` n'inclut aucun Whisper de réunion mais inclut le modèle de dictée et la séparation des voix ; `modelsFor('local', …)` inclut le modèle de réunion ; un modèle déjà installé n'est jamais proposé (reprise après échec).
- [x] **Step 2:** Échec attendu, puis `steps.ts`.
- [x] **Step 3:** `Onboarding.svelte` plein écran (sans barre latérale), une étape à la fois, indicateur de progression (points), « Retour » et « Passer » sur chaque étape, « Continuer » en action principale ; transition horizontale de 240 ms entre étapes.
  - **Bienvenue** : chouette `idle` grande, titre, une phrase, la promesse locale.
  - **Micro** : `startMicTest` à l'entrée, `stopMicTest` à la sortie et au démontage ; barre de niveau ; ✦ s'anime la première fois que le niveau dépasse 0,3 (« Je t'entends ») ; erreur `capture.*` : texte d'aide et étape passable.
  - **Modèles** : deux cartes de choix (« Sur mon ordinateur », « En ligne pour les réunions ») ; le choix en ligne ouvre la feuille Mistral ; liste des modèles à télécharger avec tailles et progression (`events.modelProgress`), téléchargés un par un ; un échec s'arrête et affiche « Réessayer ». Le choix enregistre `meeting.model` et `processing_mode` via `modeFor`.
  - **Test de dictée** : `ShortcutInput` pour le raccourci de dictée (enregistré dans les réglages), zone de texte focalisée, consigne « Garde le curseur ici, appuie sur ton raccourci et parle » ; `events.dictationDone` (ou le changement de contenu de la zone) déclenche le succès avec la chouette `success`.
  - **Tour** : trois panneaux (notes pendant l'appel, relecture des passages douteux, envoi et résumé), chacun avec une petite illustration en SVG dans le monde (lignes, ✦), son raccourci affiché (`accelerator.ts`).
  - **Fin** : chouette `success`, « Tout est prêt », bouton vers l'accueil ; enregistre `onboarding_done = true`.
- [x] **Step 4:** `App.svelte` : au démarrage, `shouldOnboard` décide (`mark_done` enregistre le réglage sans rien montrer) ; route `onboarding` sans barre latérale. Supprimer `ModelSetup.svelte`.
- [x] **Step 5:** Chaînes `onboarding.*` en fr (tutoiement) et en, `fr-typography` puis relecture des octets.
- [x] **Step 6:** Vérifications UI, commit `add first-run onboarding`.

### Task 7: Revue visuelle et DESIGN.md (contrôleur)

- [x] **Step 1:** Lancer `cd ui && bun run dev` (mode navigateur avec `mock.ts`), capturer chaque écran en clair et en sombre à 1440×900 et 900×700 dans `.impeccable/review/` (Playwright). Lancer `impeccable detect --json` sur les fichiers changés.
- [x] **Step 2:** `impeccable-finish-reviewer` avec le contrat de direction, les captures, `refs.md` ; un seul lot de corrections, une seule recapture.
- [x] **Step 3:** `review-ui` sur les captures (accessibilité, dark patterns, finition).
- [x] **Step 4:** `impeccable-documenter` écrit `DESIGN.md` et `.impeccable/design.json` depuis le build.
- [x] **Step 5:** Mettre à jour `docs/project/handoff.md` et `decisions.md`.
