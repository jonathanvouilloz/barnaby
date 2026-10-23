# Finitions dictée et destinations, plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** six retours de Jon après la v0.3.0 sous Windows : overlay visible, son de démarrage, saisie de raccourci, choix du dossier, webhook clé en main, texte gardé au presse-papiers.

**Architecture:** changements locaux dans l'existant ; un seul ajout de dépendance (plugin officiel `tauri-plugin-dialog`), plus `windows-sys` (déjà dans `Cargo.lock`) en dépendance directe Windows.

**Tech Stack:** Tauri 2, Rust, Svelte 5, cpal 0.18, hound.

**Spec:** demandes de Jon du 2026-10-23 (conversation), choix tranchés :
- son au démarrage seulement, fichier `sound.mp3` de Jon, désactivable ;
- presse-papiers : le texte dicté y reste toujours (plus de restauration) ; notification « texte copié » si aucune fenêtre n'est active ;
- raccourci : une touche seule autorisée (touches non imprimables), avec avertissement ;
- webhook : bouton « Tester » et secret généré automatiquement.

## Global Constraints

- Interface en français et anglais, via `ui/src/lib/i18n.ts` (les deux langues dans le même commit), tutoiement, phrases simples.
- Types partagés : `crates/core/src/model.rs` et `ui/src/lib/api.ts` modifiés ensemble.
- Pas de `unwrap()` hors des tests ; journaux sans contenu (ni texte dicté, ni URL complète, ni secret).
- Mode `local` : aucune requête réseau pendant le traitement. Le test du webhook n'a lieu que sur un clic de l'utilisateur.
- Secret webhook : trousseau de l'OS uniquement (`set_secret(WEBHOOK_SECRET_USER, …)` existant).
- Commandes de contrôle : `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p barnaby-core`, `cd ui && bun run check && bun test`. Sourcer `~/sysroot/env.sh` avant, un seul cargo à la fois.

## Review Focus

1. Raccourci saisi pendant que les raccourcis globaux sont suspendus, puis fenêtre fermée ou perte du focus : ils doivent revenir (test dans la tâche 2).
2. Périphérique de sortie à 44,1 kHz alors que le son est à 48 kHz : même hauteur, même durée (test de rééchantillonnage, tâche 1).
3. URL de webhook injoignable ou en 404 au clic sur « Tester » : réponse en moins de 10 s avec un code lisible, pas 4 tentatives (tâche 4).
4. Dossier choisi puis annulation du sélecteur : le chemin précédent reste (tâche 3).
5. Dictée pendant qu'aucune fenêtre n'est active, sous Linux sans `xdotool` : pas de notification erronée (fonction renvoie « inconnu », tâche 5).

---

### Task 1: Son de démarrage (fichier de Jon) et overlay au premier plan

Le travail overlay est déjà dans l'arbre de travail, non commité : `src-tauri/src/overlay.rs` (remise au premier plan à chaque `show`, clics traversants) et le module `crates/engine/src/cue.rs` (deux notes synthétisées) branché dans `src-tauri/src/dictation.rs`.

**Files:**
- Create: `crates/engine/assets/start.wav` (depuis `sound.mp3` à la racine, à supprimer ensuite)
- Modify: `crates/engine/src/cue.rs`, `src-tauri/src/dictation.rs`, `ui/src/lib/i18n.ts` (`settings.sound_cues*`)

**Interfaces:**
- Produces: `barnaby_engine::cue::play_start()` (bloque jusqu'à la fin du son, erreurs journalisées), `cue::MIC_MUTE: Duration`.

- [x] **Step 1: Convertir le son.** Le son utile est entre 0,059 s et 0,098 s du mp3, le reste est du silence.
  Run: `ffmpeg -i sound.mp3 -ss 0.05 -t 0.12 -ac 1 -ar 48000 -c:a pcm_s16le -af "afade=t=out:st=0.10:d=0.02" crates/engine/assets/start.wav`
  Expected: `ffprobe` donne ~0,12 s, mono, 48 kHz ; écoute : même son que le mp3.
- [x] **Step 2: Test qui échoue** dans `cue.rs` : `start_sound_decodes` (non vide, mono, `|x| <= 1`), `resample_keeps_duration` (`resample(&s, 48_000, 44_100).len()` ≈ `s.len() * 44_100 / 48_000` à 1 échantillon près).
- [x] **Step 3: Implémenter.** `include_bytes!("../assets/start.wav")` décodé avec `hound::WavReader::new(Cursor)` ; `fn resample(s: &[f32], from: u32, to: u32) -> Vec<f32>` par interpolation linéaire ; supprimer `tone`, `Cue`, `RISING`, l'ancien test. `MIC_MUTE` = 160 ms. Garder un gain de lecture (`VOLUME` = 0,5, à ajuster à l'oreille par Jon).
- [x] **Step 4: Retirer le son d'arrêt** dans `dictation.rs` (bloc `End::Stop` de `end`), appeler `cue::play_start()` au démarrage. Textes : « Faire un petit son au démarrage de la dictée » / « Play a short sound when dictation starts ».
- [x] **Step 5: Vérifier.** Tests engine `cue`, clippy, `bun run check`. Supprimer `sound.mp3`.
- [x] **Step 6: Commit** `add dictation start sound, raise overlay on each dictation`. Demander à Jon la source du son et la noter dans `docs/project/decisions.md` (licence).

### Task 2: Saisie du raccourci

Cause du bug : le raccourci actuel est déjà enregistré globalement ; le système le capte avant la webview, la case ne reçoit jamais la touche.

**Files:**
- Modify: `src-tauri/src/shortcuts.rs`, `src-tauri/src/commands.rs` (+ enregistrement dans `lib.rs`), `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`, `ui/src/lib/ShortcutInput.svelte`, `ui/src/lib/accelerator.ts`, `ui/src/lib/lib.test.ts`, `ui/src/lib/i18n.ts`, `ui/src/screens/Settings.svelte`

**Interfaces:**
- Produces: commandes `suspend_shortcuts()` (`unregister_all`), `resume_shortcuts(app)` (`shortcuts::register(&app.handle, &app.settings())`) et `check_shortcut(accel: String) -> bool` (`true` si une autre application l'a déjà pris : tentative d'enregistrement puis désenregistrement, raccourcis de Barnaby suspendus) ; `api.checkShortcut(accel)` ; `api.suspendShortcuts()`, `api.resumeShortcuts()` ; `shortcutWarning(accel: string): 'single_key' | 'common' | null` dans `accelerator.ts`.

- [x] **Step 1: Tests qui échouent** dans `lib.test.ts` :
  - `acceleratorFromKey` accepte seuls `F1`–`F24`, `Pause`, `Insert`, `ScrollLock`, `PrintScreen` ; renvoie toujours `'need_modifier'` pour `KeyA`, `Digit1`, `Space`, `Enter`.
  - `shortcutWarning('F8') === 'single_key'` ; `shortcutWarning('Ctrl+P') === 'common'` (liste : Ctrl+ A C V X Z Y S P O N W T F R Q, et la même chose avec Super sous macOS) ; `shortcutWarning('Ctrl+Shift+Space') === null`.
- [x] **Step 2: Implémenter** `accelerator.ts`. Vérifier dans la source de `global-hotkey` installée que `Pause`, `Insert`, `ScrollLock`, `PrintScreen` sont analysés ; retirer de la liste ceux qui ne le sont pas.
- [x] **Step 3: Suspendre pendant la saisie.** `ShortcutInput` appelle `suspendShortcuts()` à l'entrée en saisie et `resumeShortcuts()` dans `stop()` (Échap, blur, touche validée). Test : le mock compte les appels ; après blur, autant de `resume` que de `suspend`.
- [x] **Step 3b: Conflit avec une autre application.** Après chaque combinaison validée, `checkShortcut` ; si `true`, message `settings.sc.taken` (« Ce raccourci est déjà pris par une autre application. Choisis-en un autre. ») et la valeur n'est pas retenue. Les raccourcis internes aux applis (Ctrl+P dans Word) ne sont détectables sur aucun système : c'est le rôle de `shortcutWarning`. Wayland : non fiable, pas de message.
- [x] **Step 4: Textes.** Sous la case : l'avertissement de `shortcutWarning` (`settings.sc.warn_single_key` : la touche ne marchera plus pour autre chose dans aucune appli ; `settings.sc.warn_common` : « {keys} sert déjà à {action} dans la plupart des applis : il ne marchera plus là-bas »). Réécrire `settings.sc.hint` et `settings.sc.need_modifier` : une touche Ctrl, Alt, Maj ou Win + une lettre ; conseillé : Ctrl + Maj + Espace ; Échap pour annuler.
- [x] **Step 5: Vérifier** `bun test`, `bun run check`, clippy, capture des Réglages (mock).
- [x] **Step 6: Commit** `fix shortcut capture, allow single keys with warnings`.

### Task 3: Choisir le dossier de destination

**Files:**
- Modify: `src-tauri/Cargo.toml` (`tauri-plugin-dialog`), `src-tauri/src/lib.rs` (`.plugin(tauri_plugin_dialog::init())`), `src-tauri/capabilities/default.json` (`dialog:allow-open`), `ui/package.json` (`@tauri-apps/plugin-dialog`, même version mineure que le crate), `ui/src/screens/Settings.svelte`, `ui/src/lib/mock.ts`, `ui/src/lib/i18n.ts`

- [x] **Step 1: Lire la doc officielle** du plugin dialog v2 (`open({ directory: true, defaultPath })`) pour la version installée.
- [x] **Step 2: Bouton « Parcourir… »** à droite du champ (le champ reste éditable). `defaultPath` = chemin actuel, sinon `documentDir()` de `@tauri-apps/api/path`. Annulation (`null`) : ne rien changer.
- [x] **Step 3: Dossier proposé** : en activant l'interrupteur avec un chemin vide, remplir `<Documents>/Barnaby`. `write_folder` crée-t-il le dossier ? Le vérifier dans `crates/core/src/export.rs`, sinon `create_dir_all` avant écriture, avec test core.
- [x] **Step 4: Mock** : `open` renvoie `/home/demo/Documents/Barnaby`. Vérifier `bun run check`, clippy, capture.
- [x] **Step 5: Commit** `add folder picker for the folder destination`.

### Task 4: Webhook clé en main

**Files:**
- Modify: `crates/core/src/destination.rs`, `src-tauri/src/commands.rs` (+ `lib.rs`), `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`, `ui/src/screens/Settings.svelte`, `ui/src/lib/i18n.ts`

**Interfaces:**
- Produces: champ `WebhookDestination::attempts: u32` (4 via `new`) ; `pub fn sample_doc() -> MeetingDoc` (réunion d'exemple, titre « Test Barnaby », un segment, id fixe) ; commandes `test_webhook(url: String) -> CmdResult<()>` (une tentative, délai 10 s, secret du trousseau, erreur `http_<code>` ou `transport`) et `generate_webhook_secret() -> CmdResult<String>` (64 caractères hexadécimaux : deux `Uuid::new_v4().simple()`, stocké dans le trousseau, renvoyé pour copie).

- [x] **Step 1: Test core qui échoue** : `webhook_test_single_attempt` contre un serveur local qui répond 500 : une seule requête reçue, erreur `webhook_failed:http_500` ; le corps signé vérifie avec `sign`.
- [x] **Step 2: Implémenter** `attempts` et `sample_doc`, puis les deux commandes.
- [x] **Step 3: Interface.** Bouton « Tester » à côté de l'URL (désactivé si l'URL est vide) : état en cours, puis « Reçu » ou le code d'erreur expliqué. Secret : bloc repliable « Avancé : signature » avec « Générer un secret » puis « Copier » ; saisie manuelle conservée dedans.
- [x] **Step 4: Vérifier** `cargo test -p barnaby-core`, clippy, `bun run check`, capture.
- [x] **Step 5: Commit** `add webhook test button and generated secret`.

### Task 5: Le texte dicté reste au presse-papiers

**Files:**
- Modify: `src-tauri/src/inject.rs`, `src-tauri/src/dictation.rs`, `src-tauri/Cargo.toml` (`windows-sys` 0.59, `Win32_UI_WindowsAndMessaging`, cible Windows), `crates/core/src/model.rs`, `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`, `ui/src/screens/Settings.svelte`, `ui/src/lib/i18n.ts`

**Interfaces:**
- Produces: `inject::paste_text(text, delays)` sans restauration ; `inject::no_target() -> Option<bool>` (`Some(true)` = aucune fenêtre active, `None` = inconnu) ; `Alert::Copied` dans `dictation.rs`.

- [x] **Step 1: Retirer la restauration** dans `paste_text` et le champ `paste_delay_after_ms` partout (modèle, `Delays::before_restore`, api, mock, Réglages, textes `settings.delay_after`). Serde ignore le champ dans les réglages déjà enregistrés : le vérifier par un test core qui relit un JSON contenant `paste_delay_after_ms`.
- [x] **Step 2: `no_target`.** Windows : `GetForegroundWindow()` nul, ou classe (`GetClassNameW`) `Progman`, `WorkerW`, `Shell_TrayWnd` → `Some(true)`. Linux : `xdotool getactivewindow` en échec avec `xdotool` présent → `Some(true)`, sinon `None`. macOS : `None`. Appeler avant le collage.
- [x] **Step 3: Notification** `Alert::Copied` : « Texte copié, tu n'as plus qu'à coller. » / « Text copied, just paste it. », via `alert` existant.
- [x] **Step 4: Vérifier** clippy, `cargo test -p barnaby-core`, `bun run check && bun test`. Le cas Windows ne se vérifie que sur la machine de Jon.
- [x] **Step 5: Commit** `keep dictated text on the clipboard, notify when nothing is focused`.

### Task 6: Contrôle sur Windows

- [x] **Step 1:** `fr-typography` sur les nouveaux textes, `review-ui` sur les Réglages modifiés.
- [x] **Step 2:** build Windows par la CI (push sur `phase-1`), artefact donné à Jon. Pas de tag ni de release sans son accord.
- [ ] **Step 3:** Jon vérifie : pastille devant une fenêtre « toujours devant » sans voler le focus, son audible et absent du texte, Ctrl+P ressaisi, sélecteur de dossier, « Tester », dictée sur le bureau puis Ctrl+V.
