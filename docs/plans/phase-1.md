# Plan — Phase 1 (MVP local, Windows + Linux)

Périmètre : spec section 21, Phase 1. Dictée complète, réunion (capture micro + système, traitement local, relecture), participants saisis à la main, envoi vers un dossier et un webhook. Raccourcis globaux et CLI `--toggle` inclus, parce qu'ils servent la dictée. Hors périmètre : overlay de notes (Phase 2 ; les notes se saisissent dans l'écran d'enregistrement), mode API, calendrier, enrôlement vocal, signature.

Contrat figé avant de lancer les lots : `crates/core/src/model.rs`, `crates/core/src/provider.rs`, `ui/src/lib/api.ts`. Un lot qui a besoin de le modifier le signale, il ne le change pas en silence.

## Lot A — `barnaby-core` (logique pure, testée sur le VPS)
- `store` : SQLite (rusqlite, `bundled`), migrations versionnées (`PRAGMA user_version`). Tables : meetings, participants, segments, notes, steps (meeting_id, step, output JSON), review_items, dictations, speaker_names (corrections de noms, réutilisées : spec section 13), settings (une ligne JSON). CRUD, assemblage de `MeetingDoc`, suppression totale d'une réunion.
- `audio` : lecture d'un WAV vers f32 16 kHz mono (rééchantillonnage linéaire si besoin), réparation d'un WAV non finalisé, mixage de deux pistes, énergie RMS par fenêtre.
- `pipeline` : `Pipeline::run(meeting_id)` enchaîne `Step::ALL` et saute les étapes déjà enregistrées dans `steps`. Identify : piste micro dominante sur un tour = l'utilisateur (« me »), sinon « Speaker A/B… », plus les noms connus via speaker_names. Fusion des tours de parole et des segments par recouvrement maximal. États : processing → needs_review, ou failed avec un code. Si le LLM est désactivé, `summary` vaut null. `resume_pending()` au démarrage.
- `review` : détection des zones d'ombre (spec section 13) : confiance sous le seuil, chevauchement de plus de 1 s (les deux pistes actives en même temps), segment sans locuteur, inaudible (segment vide ou énergie sous un plancher sur une zone de parole). Application d'une `Resolution`.
- `export` : `<id>.json` (MeetingDoc) et `<id>.md` lisible.
- `destination` : `FolderDestination`, `WebhookDestination` (POST JSON, `X-Signature: sha256=<hex HMAC-SHA256 du corps>`, `Idempotency-Key`, 4 tentatives avec backoff exponentiel, ureq). Test de la signature.
- `llm` : client compatible OpenAI (`/chat/completions`, `response_format` json_schema) qui implémente `Structurer`.
- `net` : `NetPolicy` ; en mode `local`, refuse les hôtes hors loopback. Test : le pipeline complet en mode local, avec un LLM pointé vers un hôte distant, échoue sans émettre de requête.
- `retention` : purge de l'audio selon `AudioRetention`.

## Lot B — `barnaby-engine` (natif, compilé en CI)
- `capture` : `Recorder::start(dir)` écrit `mic.wav` et `system.wav` en continu (16 kHz mono i16, en-tête rafraîchi toutes les ~2 s), expose les niveaux RMS et un avertissement « système silencieux depuis 30 s ». Micro : cpal. Système : loopback WASAPI sous Windows (cpal, périphérique de sortie par défaut ouvert en entrée), `parec --device=@DEFAULT_MONITOR@ --format=s16le --rate=16000 --channels=1` sous Linux.
- `mic` : capture courte en mémoire pour la dictée (start/stop → Vec<f32> 16 kHz).
- `vad` : Silero (crate `voice_activity_detector`), pour retirer les silences de la dictée.
- `whisper` : `WhisperTranscriber` (whisper-rs), confiance = moyenne des probabilités des tokens, langue auto ou forcée. Une instance par modèle chargé.
- `diarize` : `SherpaDiarizer` (sherpa-rs : segmentation pyannote ONNX + embeddings).
- `models` : catalogue (whisper tiny/base/small/large-v3-turbo GGML depuis Hugging Face ; segmentation et embedding sherpa ; Silero si nécessaire), téléchargement avec progression, vérification de la taille, présence sur disque.

## Lot C — `ui/` (Svelte 5, testé sur le VPS avec un faux backend)
Écrans : accueil (dictées et réunions avec statut), enregistrement (titre, participants, niveaux, notes, arrêt), relecture (transcript à gauche, zones d'ombre à droite, lecteur audio synchronisé, renommage des locuteurs, valider et envoyer), réglages (mode de dictée, modèles et téléchargements, raccourcis, destinations, secret webhook, LLM, rétention, langue, seuil de confiance), rappel de consentement avant la première réunion. i18n FR/EN. Chouette provisoire.

## Lot D — `src-tauri` (intégration, après A)
État de l'app, commandes de `api.ts`, raccourcis globaux (maintien ou bascule), contrôleur de dictée (micro → VAD → whisper → injection), injection (presse-papiers + collage simulé, restauration ; xdotool/wtype sous Linux s'ils sont présents, sinon enigo), icône de tray qui change d'état, notification de fin de traitement, single-instance + `--toggle`/`--cancel`, file à priorité (la dictée passe devant), reprise au démarrage. CI GitHub Actions Windows + Ubuntu.

## Vérification
- VPS : `cargo test -p barnaby-core`, `bun run check`, `bun test`, `bun run build`.
- CI : `cargo clippy` + build complet sur les deux OS, et artefacts d'installation.
- Poste du propriétaire : la checklist de `docs/project/handoff.md`.
