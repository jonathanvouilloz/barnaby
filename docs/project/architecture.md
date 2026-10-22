# Architecture — Barnaby

Produit et règles fonctionnelles : `docs/SPEC.md`. Ce document fixe les frontières et les invariants du code.

## Frontières

```
ui/ (Svelte)  ──invoke/events──►  src-tauri (app)  ──►  barnaby-engine  ──►  barnaby-core
                                   shortcuts, tray,       capture, VAD,        types, SQLite,
                                   dictée, injection,     whisper, parakeet,   pipeline, relecture,
                                   overlay, commandes     sherpa, modèles      vocabulaire, export,
                                                                               webhook, LLM
```

- `barnaby-core` ne dépend d'aucune bibliothèque native (hors SQLite embarqué). Il définit les traits `Transcriber`, `Diarizer`, `Structurer` et `Destination`, et le pipeline les consomme. Tout se teste avec des doublures.
- `barnaby-engine` implémente `Transcriber` (whisper.cpp pour les réunions, Parakeet via sherpa-onnx pour la dictée) et `Diarizer` (sherpa-onnx). Il capture l'audio, découpe la dictée en flux (`LiveDictation` : VAD Silero, transcription de chaque segment dès sa fin) et gère les modèles. Il ne connaît ni Tauri ni SQLite.
- `src-tauri` câble l'ensemble : il possède l'état de l'app, les raccourcis, la dictée, l'injection et les commandes exposées au front.
- `ui/` ne parle qu'au backend, via les commandes et les événements listés dans `ui/src/lib/api.ts`. Ce fichier est le contrat. Toute modification se fait des deux côtés.

## Données

Répertoire de données de l'app (`app_data_dir` de Tauri) :

```
barnaby.db                  SQLite : meetings, steps, review_items, participants, dictations, corrections, settings
meetings/<id>/mic.wav       16 kHz mono, écrit en continu pendant la capture
meetings/<id>/system.wav    idem, piste système
models/                     modèles GGML et ONNX (téléchargés ou déposés à la main)
```

## Invariants

1. **Audio sur disque dès la capture.** Le WAV est écrit au fil de l'eau. Au démarrage, un WAV dont l'en-tête n'a pas été finalisé est réparé d'après la taille du fichier, et la réunion passe à `captured`.
2. **Étapes idempotentes.** Le résultat de chaque étape (`transcribe`, `diarize`, `identify`, `review`, `structure`) est enregistré dans `steps` (meeting_id, step, JSON). Le pipeline saute les étapes déjà faites. Au redémarrage, les réunions en `queued`, `processing` ou `sending` reprennent.
3. **Mode `local` = pas de réseau hors loopback.** `core::net::NetPolicy` refuse toute URL dont l'hôte n'est pas en loopback, ce qui couvre le LLM et les destinations pendant le traitement. Un test le vérifie. Le téléchargement de modèles passe par un autre chemin, déclenché par l'utilisateur.
4. **La dictée passe devant, et elle est transcrite pendant qu'on parle.** Les modèles de transcription sont derrière une file à priorité : une dictée prend la main entre deux blocs du traitement d'une réunion. À l'arrêt, seul le dernier segment reste à transcrire, et le modèle de dictée est préchargé au démarrage.
5. **Journaux sans contenu.** Seulement des identifiants, des états et des codes d'erreur. Jamais de texte transcrit.
6. **Un envoi raté ne perd rien.** La réunion passe à `failed`, avec la cause et « réessayer ». Le webhook envoie `Idempotency-Key: <meeting.id>`.
7. **L'overlay ne prend jamais le focus**, sinon le collage partirait dans la pastille. Sous Windows, il reste visible et on le déplace hors écran au lieu de le cacher.

## Environnements

- Dev (VPS, sans sudo) : tout compile et se teste après `. ~/sysroot/env.sh` (en-têtes extraits dans un sysroot). Pas de matériel audio.
- CI (GitHub Actions) : build et tests sur `windows-latest`, `ubuntu-22.04` et `macos-14` (Apple Silicon), avec les artefacts d'installation (`.msi`/`.exe`, `.deb`, `.dmg`). Un tag `v*` publie la release avec des noms de fichiers fixes.
- Poste du propriétaire : la seule validation réelle de la capture, de l'injection et des performances. Sur macOS, personne n'a encore testé sur un vrai Mac.
