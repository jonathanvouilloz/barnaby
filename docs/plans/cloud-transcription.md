# Transcription des réunions en ligne — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** une réunion dont le modèle est « Mistral Voxtral (en ligne) » est transcrite par l'API Mistral au lieu de whisper.cpp.

**Architecture:** un `MistralTranscriber` dans `crates/core` implémente `Transcriber` (découpe en morceaux de 600 s, envoie chaque morceau en WAV multipart, recolle les segments). `src-tauri/src/meeting.rs` le choisit quand `meeting.model == MISTRAL_MODEL`, sans passer par `MeetingTranscriber` (pas de verrou processeur). Les erreurs portent un code (`mistral_*`) que le pipeline enregistre sur la réunion. L'interface ajoute l'option, la fenêtre d'avertissement avec la clé et règle `processing_mode`.

**Tech Stack:** Rust (ureq 3.4.2 + feature `multipart`, hound, serde_json), Tauri 2, Svelte 5 + TypeScript, Bun.

**Spec:** `docs/project/cloud-transcription.md`

## Global Constraints

- Endpoint `https://api.mistral.ai/v1/audio/transcriptions`, champs multipart : `model=voxtral-mini-latest`, `timestamp_granularities=segment`, `file` (WAV 16 kHz mono, nom `audio.wav`), `context_bias` répété une fois par terme, 100 termes au plus. Jamais `language`.
- En-tête `Authorization: Bearer <clé>`. Clé dans le trousseau, identifiant `mistral-api-key`.
- Morceaux de 600 s, coupe au passage le plus calme des 5 dernières secondes.
- `confidence` des segments Mistral = 1.0. Nom du fournisseur : `mistral-voxtral` ; c'est aussi l'identifiant de modèle stocké dans `meeting.model` (`MISTRAL_MODEL`).
- Codes d'erreur : `mistral_key_missing`, `mistral_auth` (401, 403), `mistral_quota` (429), `mistral_unreachable` (réseau, délai), `mistral_failed` (autre). Journaux : codes et statuts HTTP seulement, jamais de texte transcrit.
- `NetPolicy::check` avant chaque requête (invariant 3).
- Pas de `unwrap()` hors tests. Types partagés modifiés dans `model.rs` et `ui/src/lib/api.ts` dans le même commit.
- Interface en français et en anglais (`ui/src/lib/i18n.ts`). Texte français relu puis passé au skill `fr-typography`.
- Compilation sur le VPS : `. ~/sysroot/env.sh` d'abord, `-j2`, un seul cargo à la fois. Tests : `LD_LIBRARY_PATH=$HOME/sysroot/root/usr/lib/x86_64-linux-gnu cargo test -p <crate> -j2`.

## Review Focus

1. Réponse 200 sans `segments` ou avec `segments: []` mais un `text` non vide : produire un segment unique couvrant le morceau, pour ne rien perdre (test dans la tâche 2).
2. Morceau entièrement silencieux : Mistral renvoie un texte vide ; aucun segment ajouté, pas d'erreur (test dans la tâche 2).
3. Réunion créée en mode `local` puis réglage passé sur Mistral : la requête est refusée par `NetPolicy` avant tout envoi, code `net_policy_blocked` (test dans la tâche 2).
4. Vocabulaire de plus de 100 termes ou avec des termes vides : tronqué à 100, termes vides ignorés (test dans la tâche 2).
5. Revenir de Mistral à un modèle local : `processing_mode` retombe sur `hybrid` si le résumé en ligne est actif, sinon `local` (test dans la tâche 4).

---

### Task 1: Découpage et WAV en mémoire dans le cœur

**Files:**
- Modify: `crates/core/src/audio.rs`, `src-tauri/src/sched.rs`

**Interfaces:**
- Produces: `pub fn blocks(samples: &[f32], block_s: f64) -> Vec<(usize, usize)>` et `pub fn wav_bytes(samples: &[f32]) -> anyhow::Result<Vec<u8>>` dans `barnaby_core::audio`.

- [ ] **Step 1:** Déplacer `blocks`, `SEARCH_S` et les tests `blocks_cover_audio_and_cut_in_silence` / `short_audio_is_one_block` de `sched.rs` vers `audio.rs`, en ajoutant le paramètre `block_s` (les tests passent `30.0`). Ajouter :

```rust
#[test]
fn blocks_of_600_s() {
    let b = blocks(&vec![0.1; 16_000 * 1300], 600.0);
    assert_eq!(b.len(), 3);
    assert!(b.windows(2).all(|w| w[0].1 == w[1].0));
    assert!(b[0].1 <= 600 * 16_000 && b[0].1 >= 595 * 16_000);
}

#[test]
fn wav_bytes_round_trip() {
    let bytes = wav_bytes(&[0.0, 0.5, -0.5]).unwrap();
    let r = hound::WavReader::new(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!((r.spec().sample_rate, r.spec().channels, r.len()), (16_000, 1, 3));
}
```

- [ ] **Step 2:** `cargo test -p barnaby-core audio` : échec (fonctions absentes).
- [ ] **Step 3:** Implémenter. `wav_bytes` écrit dans un `Cursor<Vec<u8>>` avec le même `WavSpec` que `write_wav` (factoriser le spec en une fonction privée). `sched.rs` appelle `audio::blocks(samples, BLOCK_S)`.
- [ ] **Step 4:** `cargo test -p barnaby-core audio` et `cargo test -p barnaby` (tests de `sched`) : verts.
- [ ] **Step 5:** Commit `move block splitting to core, add in-memory wav`.

### Task 2: Client Mistral

**Files:**
- Create: `crates/core/src/mistral.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml` (feature `multipart` de ureq), `crates/core/src/pipeline.rs` (code d'erreur), `crates/core/src/provider.rs`

**Interfaces:**
- Consumes: `audio::blocks`, `audio::wav_bytes` (tâche 1), `NetPolicy`, `net::agent`.
- Produces:
  - `pub const MISTRAL_MODEL: &str = "mistral-voxtral";`
  - `pub struct MistralTranscriber { pub api_key: String, pub vocabulary: Vec<String>, pub policy: NetPolicy }`, `impl Transcriber` (`name()` = `MISTRAL_MODEL`).
  - Dans `provider.rs` : `#[derive(Debug, thiserror::Error)] #[error("{0}")] pub struct Coded(pub &'static str);` et `pub fn error_code(err: &anyhow::Error) -> Option<&'static str>` (cherche `Coded` dans la chaîne).

- [ ] **Step 1: tests** dans `mistral.rs` (fonctions pures, pas de réseau) :

```rust
#[test]
fn parses_segments_and_offsets_them() {
    let body = r#"{"text":"a b","language":"fr","segments":[{"text":" a","start":0.5,"end":1.0},{"text":"b","start":1.2,"end":2.0}]}"#;
    let t = parse(body, 600.0, 30.0).unwrap();
    assert_eq!(t.language, "fr");
    assert_eq!(t.segments[0], RawSegment { start: 600.5, end: 601.0, text: "a".into(), confidence: 1.0 });
}
#[test]
fn text_without_segments_becomes_one_segment() { /* segments absent ou vide, text "bonjour" → 1 segment 600.0..630.0 */ }
#[test]
fn empty_text_gives_no_segment() { /* text "", segments [] → segments vide */ }
#[test]
fn bias_terms_are_capped_and_cleaned() { /* 150 termes dont des vides → 100 non vides, ordre gardé */ }
#[test]
fn status_codes_map_to_error_codes() {
    assert_eq!(status_code(401), "mistral_auth");
    assert_eq!(status_code(403), "mistral_auth");
    assert_eq!(status_code(429), "mistral_quota");
    assert_eq!(status_code(500), "mistral_failed");
}
#[test]
fn local_mode_blocks_before_sending() {
    let t = MistralTranscriber { api_key: "k".into(), vocabulary: vec![], policy: NetPolicy { mode: ProcessingMode::Local } };
    let err = t.transcribe(&[0.0; 16_000], None).unwrap_err();
    assert!(crate::net::is_blocked(&err));
}
```

Et dans `pipeline.rs` : un test où le transcripteur factice renvoie `Err(Coded("mistral_quota").into())` ; la réunion passe en `failed` avec le code `mistral_quota` (suivre le modèle des tests de pipeline existants).

- [ ] **Step 2:** `cargo test -p barnaby-core mistral pipeline` : échec.
- [ ] **Step 3: implémenter.**
  - `fn parse(body: &str, offset: f64, duration: f64) -> Result<Transcript>` (structs serde privés, champs inconnus ignorés, `segments` en `#[serde(default)]`), `fn bias_terms(vocabulary: &[String]) -> Vec<&str>`, `fn status_code(status: u16) -> &'static str`.
  - `transcribe` : `policy.check(URL)?` d'abord ; puis pour chaque `blocks(samples, 600.0)` : `Form::new().text("model", "voxtral-mini-latest").text("timestamp_granularities", "segment")`, un `.text("context_bias", t)` par terme, `.part("file", Part::bytes(&wav).file_name("audio.wav").mime_str("audio/wav")?)` ; `net::agent(Duration::from_secs(600))`, `.send(form)`. Erreur `ureq::Error::StatusCode(s)` → `Coded(status_code(s))` (log du statut) ; autre erreur de transport → `Coded("mistral_unreachable")`. Langue : la première non vide des morceaux.
  - `pipeline.rs`, dans le calcul du code d'échec : `net::BLOCKED` d'abord, puis `provider::error_code(&e)`, sinon `{step}_failed` comme aujourd'hui.
- [ ] **Step 4:** `cargo test -p barnaby-core` puis `cargo clippy -p barnaby-core --all-targets -j2 -- -D warnings` : verts et propres.
- [ ] **Step 5:** Commit `add Mistral Voxtral transcriber`.

### Task 3: Branchement côté application

**Files:**
- Modify: `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (enregistrement des commandes), `src-tauri/src/meeting.rs`

**Interfaces:**
- Consumes: `MistralTranscriber`, `MISTRAL_MODEL` (tâche 2).
- Produces: commandes Tauri `set_mistral_api_key(key: String) -> CmdResult<()>` et `has_mistral_api_key() -> bool` ; `App::mistral_api_key() -> Option<String>` ; constante `MISTRAL_API_KEY_USER = "mistral-api-key"`.

- [ ] **Step 1:** Ajouter la constante, `App::mistral_api_key()` et les deux commandes sur le modèle de `set_llm_api_key` / `has_llm_api_key`.
- [ ] **Step 2:** Dans `meeting::process`, construire le transcripteur :
  - si `settings.meeting.model == MISTRAL_MODEL` : sans clé → `fail("mistral_key_missing")` ; sinon `Arc::new(MistralTranscriber { api_key, vocabulary: settings.vocabulary.clone(), policy: NetPolicy { mode } })`, sans `MeetingTranscriber` ;
  - sinon : chemin actuel (`app.transcriber` + `MeetingTranscriber`).
- [ ] **Step 3:** `cargo clippy --workspace --all-targets -j2 -- -D warnings` et `cargo test --workspace -j2` : propres et verts.
- [ ] **Step 4:** Commit `wire Mistral transcription into meeting processing`.

### Task 4: Interface

**Files:**
- Modify: `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`, `ui/src/lib/settings.ts`, `ui/src/lib/lib.test.ts`, `ui/src/lib/i18n.ts`, `ui/src/screens/Settings.svelte`

**Interfaces:**
- Consumes: commandes de la tâche 3.
- Produces: `api.setMistralApiKey(key)`, `api.hasMistralApiKey()` ; dans `settings.ts` : `export const MISTRAL_MODEL = 'mistral-voxtral'` et `export function modeFor(meetingModel: string, summaryCloud: boolean): ProcessingMode`.

- [ ] **Step 1: test** dans `lib.test.ts` :

```ts
test('processing mode follows meeting model and summary', () => {
  expect(modeFor(MISTRAL_MODEL, false)).toBe('api');
  expect(modeFor(MISTRAL_MODEL, true)).toBe('api');
  expect(modeFor('whisper-small', true)).toBe('hybrid');
  expect(modeFor('whisper-small', false)).toBe('local');
});
```

- [ ] **Step 2:** `cd ui && bun test` : échec.
- [ ] **Step 3: implémenter.**
  - `modeFor` dans `settings.ts`. Dans `Settings.svelte`, l'état « résumé en ligne » devient `summaryCloud = $derived(!isLoopbackUrl(draft.llm.base_url))` à la place de `processing_mode !== 'local'` ; `useLocal`, `useCloud` et le changement de modèle de réunion affectent `draft.processing_mode = modeFor(draft.meeting.model, summaryCloud)`. L'avertissement `llmNotLocal` disparaît s'il ne peut plus se produire.
  - Liste des modèles de réunion : les modèles Whisper puis une option `MISTRAL_MODEL`, libellé `settings.model_mistral`.
  - Choisir Mistral sans clé enregistrée (`hasMistralApiKey()` faux) ouvre une `Sheet` dédiée : ce qui part (l'audio des réunions, le vocabulaire), chez qui (Mistral, France), le prix (environ 0,18 $ par heure), un champ clé, « Annuler » remet le modèle précédent, « Activer » enregistre la clé puis garde Mistral. Bouton « Activer » désactivé tant que le champ est vide.
  - Avec Mistral choisi et une clé enregistrée : un champ pour remplacer la clé, comme pour la clé OpenRouter.
  - `mock.ts` : les deux commandes.
  - `i18n.ts` (fr et en) : `settings.model_mistral`, les textes de la fenêtre (`mistral.*`), et `error.mistral_key_missing`, `error.mistral_auth`, `error.mistral_quota`, `error.mistral_unreachable`, `error.mistral_failed`, chacun disant quoi faire (ex. auth : « Clé Mistral refusée. Vérifiez-la dans Réglages › Réunion, puis réessayez. »).
- [ ] **Step 4:** `cd ui && bun run check && bun test` : 0 erreur, tests verts. Relire le texte français, puis skill `fr-typography` sur les nouvelles chaînes.
- [ ] **Step 5:** Commit `add Mistral option to meeting settings`.

### Task 5: Documents

**Files:**
- Modify: `docs/SPEC.md` (section 6, ligne `api` : Mistral Voxtral pour la transcription des réunions, diarisation sherpa-onnx), `docs/project/decisions.md` (entrée datée : Mistral en direct, réunions seulement, pas de basculement automatique), `docs/project/cloud-transcription.md` (la fenêtre s'ouvre tant qu'aucune clé n'est enregistrée ; le test manuel se fait sur une nouvelle réunion).

- [ ] **Step 1:** Modifier les trois fichiers.
- [ ] **Step 2:** Commit `docs: record Mistral transcription decision`.

## Vérification finale

- `cargo clippy --workspace --all-targets -j2 -- -D warnings`, `cargo test --workspace -j2`, `cd ui && bun run check && bun test`.
- Test manuel par Jon (build CI) : nouvelle réunion avec Mistral, puis clé fausse (message `mistral_auth`), puis sans réseau (`mistral_unreachable`, « Réessayer » fonctionne une fois le réseau revenu).
