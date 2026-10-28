# Suivi et accès aux réunions, plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** retours de Jon après sa première vraie réunion Mistral sous Windows (2026-10-28) : voir où en est le traitement, savoir si la clé Mistral marche, rouvrir et exporter une réunion déjà validée, ne plus valider vers « nulle part ».

**Architecture:** ajouts dans l'existant. Le pipeline reçoit un rappel de progression optionnel, que l'app relaie en événement `meeting-progress`. Une commande teste la clé Mistral. L'écran de relecture gagne un mode lecture seule avec export. L'onboarding gagne une étape « dossier ». Aucune dépendance ajoutée.

**Tech Stack:** Tauri 2, Rust (ureq 3, sherpa-rs 0.6.8), Svelte 5.

**Spec:** demandes de Jon du 2026-10-28 (conversation) :
- barre de progression du traitement, avec l'étape en cours ;
- journal qui dit quelle étape tourne et combien de temps elle a pris ;
- bouton pour vérifier que la clé Mistral fonctionne, dans l'onboarding et dans Réglages ;
- rouvrir depuis l'accueil une réunion déjà validée ou envoyée ;
- onboarding : demander le dossier où enregistrer les réunions, en Markdown et JSON (format actuel de la destination dossier) ;
- pouvoir tester que l'envoi fonctionne (le test du webhook existe déjà ; le webhook envoie le JSON `MeetingDoc`, le dossier écrit `.md` et `.json`).

Ajouts de ma part, annoncés à Jon : erreur Mistral au bout de 5 minutes au plus par morceau au lieu de 10 ; avertissement avant « Valider et envoyer » quand aucune destination n'est active.

## Global Constraints

- Interface en français et anglais, via `ui/src/lib/i18n.ts` (les deux langues dans le même commit), tutoiement, phrases simples, apostrophe typographique `’` comme le reste du fichier.
- Types partagés : `crates/core/src/model.rs` et `ui/src/lib/api.ts` modifiés ensemble.
- Pas de `unwrap()` hors des tests ; journaux sans contenu : identifiants, noms d'étape, durées, codes seulement.
- Mode `local` : aucune requête réseau pendant le traitement. Le test de la clé n'a lieu que sur un clic.
- Clé Mistral : trousseau de l'OS uniquement (`set_mistral_api_key` existant).
- Contrôles : `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p barnaby-core`, `cd ui && bun run check && bun test`. Sourcer `~/sysroot/env.sh` avant, un seul cargo à la fois.

## Review Focus

1. Traitement repris après un échec ou un redémarrage : les étapes déjà sauvées sont sautées, la progression ne doit ni reculer ni rester bloquée sur une étape sautée (test dans la tâche 1).
2. Fenêtre ouverte en cours de traitement, après des événements manqués : la ligne affiche « En traitement » sans barre jusqu'au prochain événement, sans erreur (tâche 3).
3. Clé collée avec des espaces ou un retour à la ligne : elle est nettoyée avant le test et l'enregistrement (tâche 4).
4. Réunion envoyée dont l'audio est supprimé : le mode lecture s'ouvre, minutages désactivés, export possible (tâche 5).
5. Export vers un dossier où l'on ne peut pas écrire : message d'erreur lisible, rien de perdu (tâche 5).

---

### Task 1: Progression et journal dans le pipeline (cœur)

**Files:**
- Modify: `crates/core/src/model.rs` (type `Progress`)
- Modify: `crates/core/src/provider.rs` (méthodes par défaut avec progression)
- Modify: `crates/core/src/pipeline.rs` (champ `progress`, journal par étape, tests)
- Modify: `crates/core/src/mistral.rs` (progression par morceau, journal, délai)
- Modify: `crates/engine/src/e2e.rs`, `src-tauri/src/meeting.rs` (ajouter `progress: None` pour compiler ; la tâche 2 branche le vrai rappel)
- Modify: `ui/src/lib/api.ts` (type `MeetingProgress`)

**Interfaces:**
- Produces :
  - `model.rs` : `#[derive(Debug, Clone, PartialEq, Serialize)] pub struct Progress { pub meeting_id: Uuid, pub step: Step, pub index: u8, pub total: u8, pub fraction: f32 }` ; `step` sérialisé en snake_case comme le reste ; `index` commence à 1 ; `total` vaut 5 avec un structureur, 4 sans ; `fraction` dans [0, 1].
  - `provider.rs` : `pub type ProgressFn = Arc<dyn Fn(f32) + Send + Sync>;`
  - `Transcriber::transcribe_with_progress(&self, samples: &[f32], language: Option<&str>, progress: &ProgressFn) -> anyhow::Result<Transcript>`, par défaut `self.transcribe(samples, language)`.
  - `Diarizer::diarize_with_progress(&self, samples: &[f32], progress: &ProgressFn) -> anyhow::Result<Vec<SpeakerTurn>>`, par défaut `self.diarize(samples)`.
  - `Pipeline { …, pub progress: Option<Arc<dyn Fn(Progress) + Send + Sync>> }`.
  - `api.ts` : `export interface MeetingProgress { meeting_id: string; step: 'transcribe' | 'diarize' | 'identify' | 'review' | 'structure'; index: number; total: number; fraction: number }`.

- [x] **Step 1: Write the failing tests** dans `pipeline.rs`

```rust
#[test]
fn reports_progress_per_step_in_order() {
    // Pipeline de test sans structureur, `progress` qui pousse dans un Mutex<Vec<Progress>>.
    // Après run : chaque étape (Transcribe, Diarize, Identify, Review) apparaît avec fraction 0.0
    // puis 1.0 ; index 1..=4 dans l'ordre ; total == 4 partout ; aucune fraction ne recule pour un même index.
}

#[test]
fn resumed_run_reports_skipped_steps_as_done() {
    // Premier run qui échoue à la transcription (QuotaTranscriber), puis run qui réussit avec FakeTranscriber
    // après avoir sauvé à la main la sortie de Transcribe : le second run émet Transcribe index 1 fraction 1.0
    // en premier (pas de 0.0), puis les étapes suivantes.
}
```

- [x] **Step 2: Run** `cargo test -p barnaby-core pipeline` ; attendu : erreur de compilation (`progress` inconnu).

- [x] **Step 3: Implement**
  - `Pipeline::process` : pour chaque étape de `Step::ALL` déjà sauvée, émettre `fraction: 1.0` ; pour chaque étape à faire, émettre `0.0`, lancer, émettre `1.0`. Index = position dans `Step::ALL` + 1 (Structure est la 5e et n'existe que si `total == 5`, donc ne pas l'émettre sans structureur).
  - Transcribe et Diarize appellent les variantes `_with_progress` avec un `ProgressFn` qui émet la fraction de l'étape en cours.
  - Journal : `log::info!("meeting {id} step {step:?} started")` puis `log::info!("meeting {id} step {step:?} done in {:.1}s", …)` avec `Instant` ; `log::info!("meeting {id} audio {:.0}s", durée)` après `load_audio`.
  - `MistralTranscriber` : surcharger `transcribe_with_progress` (le corps actuel y passe, `transcribe` délègue avec un rappel vide) ; après chaque morceau, `progress(fait / total)` et `log::info!("mistral block {n}/{total} done in {:.1}s")`. Délai `Duration::from_secs(300)` avec ce commentaire : `// 10 min de WAV 16 bits = 19 Mo : 300 s laissent ~1 Mb/s d'envoi plus la transcription.`
  - Ajouter `progress: None` dans `e2e.rs` et `meeting.rs`, et le type dans `api.ts`.

- [x] **Step 4: Run** `cargo test -p barnaby-core` ; attendu : tout passe, dont les deux nouveaux tests.

- [x] **Step 5: Commit** `git commit -m "report pipeline progress and step timings"`

### Task 2: Progression des moteurs et événement `meeting-progress` (app)

**Files:**
- Modify: `crates/engine/src/diarize.rs` (surcharge `diarize_with_progress`)
- Modify: `src-tauri/src/sched.rs` (surcharge dans `MeetingTranscriber`)
- Modify: `src-tauri/src/meeting.rs` (rappel qui émet l'événement)
- Modify: `ui/src/lib/api.ts` (`events.meetingProgress`)

**Interfaces:**
- Consumes : `ProgressFn`, `Progress`, `Pipeline.progress` (tâche 1).
- Produces : événement Tauri `meeting-progress` de charge `Progress` ; `events.meetingProgress(cb: (p: MeetingProgress) => void)`.

- [x] **Step 1: Implement**
  - `SherpaDiarizer::diarize_with_progress` : passer à `compute` `Some(Box::new(move |done, total| { if total > 0 { p(done as f32 / total as f32) } 0 }))`, avec un clone de l'`Arc`. Vérifier dans `sherpa-rs-0.6.8/src/diarize.rs` (fonction `progress_callback_wrapper`) que 0 veut dire « continuer » et le noter en commentaire. `diarize` délègue avec un rappel vide.
  - `MeetingTranscriber::transcribe_with_progress` : même boucle qu'aujourd'hui, `progress(fin_du_morceau / samples.len())` après chaque morceau ; `transcribe` délègue.
  - `meeting.rs::process` : `progress: Some(Arc::new(move |p| { let _ = handle.emit("meeting-progress", p); }))` avec un clone de `app.handle`.
  - Le rappel sherpa arrive par morceau de segmentation (une dizaine de secondes d'audio) : pas de limitation de débit nécessaire.

- [x] **Step 2: Run** `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test -p barnaby-engine diarize -- --ignored` (télécharge les modèles une fois) ; attendu : propre, le test passe.

- [x] **Step 3: Commit** `git commit -m "emit meeting-progress from transcription and diarization"`

### Task 3: Barre de progression sur l'accueil

**Files:**
- Modify: `ui/src/lib/state.svelte.ts` (`app.progress`, écoute)
- Modify: `ui/src/screens/Home.svelte`
- Modify: `ui/src/lib/i18n.ts`
- Modify: `ui/src/lib/mock.ts` (une réunion en traitement qui avance, pour les captures)
- Test: `ui/src/lib/lib.test.ts`

**Interfaces:**
- Consumes : `events.meetingProgress`, `MeetingProgress`.
- Produces : `app.progress: Record<string, MeetingProgress>` ; `progressLabel(p: MeetingProgress, t): string` dans `ui/src/lib/format.ts`.

- [x] **Step 1: Write the failing test** dans `lib.test.ts` : `progressLabel({ step: 'diarize', index: 2, total: 5, fraction: 0.4, … })` vaut `'Étape 2 sur 5 : séparation des voix'` en français et `'Step 2 of 5: separating voices'` en anglais.

- [x] **Step 2: Run** `cd ui && bun test` ; attendu : FAIL (`progressLabel` absent).

- [x] **Step 3: Implement**
  - Clés i18n : `progress.step` (`Étape {n} sur {total} : {label}`) et `progress.transcribe` « transcription », `progress.diarize` « séparation des voix », `progress.identify` « attribution des phrases », `progress.review` « repérage des zones d’ombre », `progress.structure` « résumé » ; en anglais : transcription, separating voices, assigning speakers, finding unclear passages, summary.
  - `state.svelte.ts` : écouter `meetingProgress` et ranger par `meeting_id` ; dans le gestionnaire `meetingUpdated` existant, supprimer l'entrée quand `state !== 'processing'`.
  - `Home.svelte` : pour une réunion `processing` avec une entrée, afficher sous l'heure le libellé et un `<progress max="1" value={fraction}>` avec `aria-label` = libellé ; sans entrée, l'affichage actuel. Style : tokens de `DESIGN.md`, piste fine, pas d'animation hors `prefers-reduced-motion: no-preference`.

- [x] **Step 4: Run** `cd ui && bun run check && bun test` ; attendu : 0 erreur, tout passe. Capture du mock (`bun run dev` puis Playwright) d'une ligne en traitement, relue avec `review-ui`.

- [x] **Step 5: Commit** `git commit -m "show processing progress on home"`

### Task 4: Vérifier la clé Mistral

**Files:**
- Modify: `crates/core/src/mistral.rs` (`check_key`)
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (commande `test_mistral_key`)
- Modify: `ui/src/lib/api.ts`, `ui/src/lib/MistralSheet.svelte`, `ui/src/screens/Settings.svelte`, `ui/src/lib/i18n.ts`, `ui/src/lib/mock.ts`

**Interfaces:**
- Produces :
  - `pub fn check_key(api_key: &str) -> anyhow::Result<()>` : `GET https://api.mistral.ai/v1/models`, en-tête `Authorization: Bearer …`, délai 15 s ; erreurs `Coded` avec les codes existants (`status_code` : `mistral_auth`, `mistral_quota`, `mistral_failed` ; transport : `mistral_unreachable`). Une fonction privée `check_key_at(url, api_key)` sert aux tests.
  - Commande `test_mistral_key(key: Option<String>) -> CmdResult<()>` : `None` teste la clé enregistrée (`mistral_key_missing` si aucune), dans `spawn_blocking` comme `test_webhook`, erreur = le code.
  - `api.testMistralKey(key?: string) => invoke<void>('test_mistral_key', { key })`.

- [x] **Step 1: Write the failing tests** dans `mistral.rs`, avec le serveur `destination::tests::serve` :

```rust
#[test]
fn check_key_maps_answers() {
    // 200 -> Ok ; 401 -> erreur dont le Coded vaut "mistral_auth" ; 429 -> "mistral_quota".
    // La requête reçue porte "authorization: Bearer k".
}
```

- [x] **Step 2: Run** `cargo test -p barnaby-core mistral` ; attendu : FAIL (`check_key_at` absent).

- [x] **Step 3: Implement** le cœur, la commande, puis l'interface :
  - `MistralSheet` : « Confirmer » teste d'abord la clé saisie (nettoyée par `trim`) ; si le test échoue, afficher `errorText(code)` sous le champ et ne rien enregistrer ; sinon enregistrer comme aujourd'hui. Libellé du bouton pendant le test : « Vérification… ».
  - `Settings.svelte`, `keyForm` pour `mistral` quand une clé est enregistrée : bouton « Tester la clé » à côté de « Effacer », avec le même schéma d'états que `hookTest` (`idle | running | ok | failed`) ; résultats « La clé fonctionne. » / `errorText(code)`.
  - Mock : `test_mistral_key` réussit, sauf pour la clé `bad` qui renvoie `mistral_auth`.

- [x] **Step 4: Run** `cargo test -p barnaby-core && cd ui && bun run check && bun test` ; attendu : tout passe.

- [x] **Step 5: Commit** `git commit -m "test the Mistral key before saving and from settings"`

### Task 5: Rouvrir et exporter une réunion validée

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (commande `export_meeting`)
- Modify: `ui/src/lib/api.ts`, `ui/src/screens/Home.svelte`, `ui/src/screens/Review.svelte`, `ui/src/lib/i18n.ts`, `ui/src/lib/mock.ts`

**Interfaces:**
- Produces :
  - Commande `export_meeting(id: String, dir: String) -> CmdResult<()>` : `export::write_folder(&db.meeting_doc(id)?, Path::new(&dir))`, erreur `export_failed` (journal : identifiant seulement).
  - `api.exportMeeting(id: string, dir: string)`.
  - `ui/src/lib/review.ts` : `export function isReadOnly(state: MeetingState): boolean`, vrai pour `reviewed`, `sending`, `sent` et pour `failed` quand l'erreur commence par `send_failed`.

- [x] **Step 1: Write the failing test** dans `lib.test.ts` : `isReadOnly('sent')`, `isReadOnly('reviewed')` vrais ; `isReadOnly('needs_review')`, `isReadOnly('processing')` faux.

- [x] **Step 2: Run** `cd ui && bun test` ; attendu : FAIL.

- [x] **Step 3: Implement**
  - `Home.svelte` : cliquable si `needs_review`, `recording` ou `isReadOnly` ; `open` mène à `review` dans les deux derniers cas.
  - `Review.svelte` en lecture seule : pas de bouton « Valider et envoyer », pas d'actions sur les zones, pas d'édition ni de renommage ; à la place, un bouton « Exporter… » qui ouvre `api.pickFolder(defaultFolder ou dossier de destination)` puis appelle `exportMeeting` et notifie « Réunion exportée en Markdown et JSON. ». Le retour mène à l'accueil, pas à la file de relecture. Sans audio (supprimé à la validation), les minutages restent désactivés comme aujourd'hui, et une ligne discrète dit « L’audio a été supprimé après la validation. ».
  - Mock : une réunion `sent` avec segments et sans `audio_path`.

- [x] **Step 4: Run** `cd src-tauri && cargo clippy --all-targets -- -D warnings` puis `cd ui && bun run check && bun test` ; attendu : propre. Captures du mock (réunion envoyée ouverte), relues avec `review-ui`.

- [x] **Step 5: Commit** `git commit -m "open sent meetings read-only and export them"`

### Task 6: Dossier à l'onboarding et avertissement sans destination

**Files:**
- Create: `ui/src/onboarding/Destination.svelte`
- Modify: `ui/src/onboarding/steps.ts`, `ui/src/onboarding/Onboarding.svelte`, `ui/src/screens/Review.svelte`, `ui/src/lib/i18n.ts`
- Test: `ui/src/lib/lib.test.ts`

**Interfaces:**
- Consumes : `api.defaultFolder`, `api.pickFolder` (existants).
- Produces : `STEPS = ['welcome', 'mic', 'models', 'destination', 'dictation', 'tour', 'done']` ; `export function hasDestination(d: Settings['destinations']): boolean` dans `ui/src/lib/settings.ts` (dossier actif avec chemin, ou webhook actif avec URL, comme `from_settings` côté Rust).

- [x] **Step 1: Write the failing tests** : `STEPS[3] === 'destination'` ; `hasDestination` faux pour les réglages par défaut, vrai avec `folder: { enabled: true, path: 'C:\\x' }`, faux avec `folder: { enabled: true, path: '' }`, vrai avec un webhook actif et une URL.

- [x] **Step 2: Run** `cd ui && bun test` ; attendu : FAIL.

- [x] **Step 3: Implement**
  - `Destination.svelte` : texte « Chaque réunion validée est enregistrée dans ce dossier, en Markdown (lisible) et en JSON (pour d’autres outils). » ; interrupteur actif par défaut ; chemin prérempli avec `defaultFolder()` ; bouton « Parcourir… » ; écrit directement dans `app.settings.destinations.folder` (l'onboarding enregistre `app.settings` à la fin). « Passer » laisse la destination désactivée.
  - `Review.svelte` : au clic sur « Valider et envoyer », si `!hasDestination(app.settings.destinations)`, ouvrir un `Sheet` : titre « Aucune destination », texte « La réunion restera seulement dans Barnaby. Tu pourras l’ouvrir et l’exporter depuis l’accueil. », boutons « Choisir un dossier » (mène aux Réglages, section `s-dest`) et « Valider quand même ».

- [x] **Step 4: Run** `cd ui && bun run check && bun test` ; attendu : tout passe. Capture de l'étape (mock en onboarding) et de la fenêtre d'avertissement, relues avec `review-ui`.

- [x] **Step 5: Commit** `git commit -m "ask for a destination folder at onboarding and warn before sending nowhere"`

### Task 7: Vérification d'ensemble et version

- [x] **Step 1:** les quatre contrôles des Global Constraints, propres.
- [x] **Step 2:** relecture à part de chaque nouveau texte visible (FR et EN) avec `copy-editing`.
- [x] **Step 3:** mettre à jour `docs/project/handoff.md` (ce qui reste à tester par Jon sous Windows : progression réelle d'une réunion Mistral de 30 min, test de clé, export, onboarding sur une installation neuve).
- [x] **Step 4:** release seulement avec l'accord de Jon.
