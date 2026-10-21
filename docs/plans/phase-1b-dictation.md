# Plan — Itération 1b : dictée instantanée, vocabulaire, IA cloud, overlay

Retour du propriétaire après test sous Windows : la dictée est trop lente, le raccourci ne réagit pas (un appui court en mode maintien enregistre environ 0,1 s, que la VAD jette), il n'y a aucun retour visuel, et pas de vocabulaire personnalisé. Référence : Willow transcrit et colle environ 1 min de dictée en 0,34 s.

Mesures sur le VPS (2 vCPU, sans GPU), moteurs déjà chargés : whisper-base met 6,1 s pour 5 s de parole et 16,8 s pour 60 s ; Parakeet v3 int8 (sherpa-onnx) met 0,7 s et 6,2 s, avec un texte français identique et ponctué. Le chargement de Parakeet prend environ 6,6 s.

Contrat déjà figé (commit de ce plan) :
- `Settings.vocabulary` et `Settings.cloud_disclosure_acknowledged` ;
- `DictationSettings.show_overlay` ; dictée par défaut en `toggle` avec le modèle `parakeet-v3` ;
- `ModelKind` `parakeet` ;
- commandes `set_llm_api_key` / `has_llm_api_key` ;
- événement `dictation-level`.

## Lot E — cœur (`crates/core`)
- `vocabulary` : `correct(text, &[String]) -> String`. Remplace les suites de 1 à 3 mots proches d'un terme du vocabulaire (distance normalisée sur une forme sans accents ni casse, seuil prudent) par la graphie exacte du terme. Ne touche jamais un mot court courant. Tests sur des cas réels (« barbor concept » → « Barber Concept »), et sur des faux positifs qu'on refuse.
- Pipeline : applique `correct` aux segments à l'étape Identify (ou juste après Transcribe), avec le vocabulaire des réglages.
- `llm` : le prompt inclut le vocabulaire ; `api_key` est envoyée en `Authorization: Bearer`. Les en-têtes OpenRouter facultatifs (`HTTP-Referer`, `X-Title: Barnaby`) sont acceptés. Le mode `hybrid` autorise l'hôte distant (c'est déjà le comportement de `NetPolicy`) : test à l'appui.

## Lot F — moteur et app (`crates/engine`, `src-tauri`, CI)
- Moteur : `ParakeetTranscriber` (sherpa-rs `TransducerRecognizer`, `model_type = "nemo_transducer"`, greedy) qui implémente `Transcriber`. Entrée de catalogue `parakeet-v3` (archive `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2`, sha256 `5793d0fd397c5778d2cf2126994d58e9d56b1be7c04d13c7a15bb1b4eafb16bf`, plusieurs fichiers à extraire : encoder, decoder, joiner, tokens).
- Dictée en continu : pendant l'enregistrement, VAD Silero en flux. Chaque segment de parole terminé (pause d'environ 0,5 s, ou 20 s au maximum) est transcrit aussitôt en arrière-plan, avec la priorité dictée. À l'arrêt, seul le dernier segment reste à transcrire. Ensuite : assemblage, correction du vocabulaire, collage. Objectif : moins de 0,5 s après l'arrêt sur un PC de bureau, pour 30 à 60 s de dictée.
- Préchargement du modèle de dictée au démarrage (fil d'arrière-plan) et après un changement de réglage.
- `dictation-level` émis environ 20 fois par seconde pendant la dictée.
- Overlay : fenêtre Tauri `overlay` (sans bordure, transparente, toujours au-dessus, hors barre des tâches, **jamais focalisée**), environ 200×56, en bas au centre de l'écran principal. Elle charge `index.html#overlay`, apparaît au début de la dictée si `show_overlay` est actif, et disparaît à la fin. Capabilities à jour.
- Commandes `set_llm_api_key` / `has_llm_api_key` (trousseau). Le worker de réunion passe la clé à `OpenAiStructurer`.
- CI : whisper.cpp portable (`GGML_NATIVE=OFF`, AVX/AVX2/FMA/F16C activés), pour ne plus compiler pour le CPU du serveur de la CI.

## Lot G — interface (`ui/`)
- Overlay (`#overlay`) : pastille arrondie translucide, petite chouette « à l'écoute » et onde de 5 à 7 barres pilotée par `dictation-level`. État « transcription » bref. Fond transparent, aucun texte superflu, mouvement coupé avec `prefers-reduced-motion`.
- Réglages :
  - Dictée : modèle (Parakeet recommandé, Whisper possible), mode (bascule par défaut), overlay oui/non.
  - Vocabulaire : liste éditable de termes, avec ajout et suppression.
  - Structuration : « Locale (Ollama) » ou « Cloud (OpenRouter) ». En cloud : clé API en écriture seule, modèle libre (exemples : `qwen/qwen3-…`, `deepseek/…`, `z-ai/glm-…`), et `processing_mode` réglé sur `hybrid`. Avant la première activation, une feuille explique ce qui part (texte du transcript, jamais l'audio), chez qui (OpenRouter et le fournisseur du modèle), et l'enregistre dans `cloud_disclosure_acknowledged`.
- Mock à jour. Feuille de premier lancement compatible avec `parakeet`.

## Vérification
`cargo test`, clippy, `bun run check` / `bun test`, mesure de latence Parakeet en continu sur le VPS, captures de l'overlay et des réglages, puis CI Windows et test sur le PC du propriétaire.
