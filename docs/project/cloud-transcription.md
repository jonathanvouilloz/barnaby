# Transcription des réunions en ligne (Mistral Voxtral)

Décision de Jon, 2026-10-22.

## Contexte

Test réel : l'audio d'une réunion est net, mais la transcription par Whisper small (modèle par défaut des réunions) est inutilisable. La dictée, sur Parakeet v3, fonctionne bien. La qualité locale dépend aussi de la machine : un ordinateur peu puissant transcrit lentement ou mal.

Objectif : proposer une transcription en ligne pour les réunions, au choix de l'utilisateur, avec Mistral Voxtral Mini Transcribe (0,003 $ par minute, soit environ 0,18 $ par heure de réunion).

## Périmètre

- Réunions seulement. La dictée reste locale (Parakeet ou Whisper).
- Fournisseur unique : l'API Mistral en direct, avec sa propre clé. Ni OpenRouter, ni Deepgram (cité par la spec section 6, remplacé ici).
- Seule l'étape de transcription change. Diarisation (sherpa-onnx, locale), relecture, résumé et envoi restent tels quels.

Hors périmètre : la diarisation par Mistral (`diarize`, format de réponse non documenté), le basculement automatique vers le local en cas d'échec, la transcription en ligne de la dictée.

## Ce que voit l'utilisateur

1. Réglages › Réunion : la liste des modèles de transcription gagne l'entrée « Mistral Voxtral (en ligne) ».
2. Tant qu'aucune clé n'est enregistrée, chaque choix de cette entrée ouvre la fenêtre d'avertissement existante (`cloud.*`, `Settings.svelte`), avec un texte propre à ce cas : l'audio des réunions part chez Mistral (France), à environ 0,18 $ par heure. Elle demande la clé Mistral.
3. La clé va dans le trousseau de l'OS, sous un nouvel identifiant `mistral-api-key`, par la commande existante d'enregistrement de secret.
4. Ce choix met `processing_mode` à `api`. Revenir à un modèle local remet `hybrid` si le résumé en ligne est actif, sinon `local`. Une réunion garde le mode de sa création (comportement actuel).
5. L'écran « Modèles à installer » ne réclame pas de téléchargement pour cette entrée.

## Fonctionnement

- Nouveau `MistralTranscriber` dans `crates/core` (à côté de `llm.rs`, qui fait déjà des appels HTTP sous `NetPolicy`). Il implémente `Transcriber`.
- Appel : `POST https://api.mistral.ai/v1/audio/transcriptions`, multipart, `model=voxtral-mini-latest`, `timestamp_granularities=segment`, fichier WAV 16 kHz mono encodé en mémoire.
- Le réglage `language` est ignoré pour ces réunions : Mistral refuse `language` avec `timestamp_granularities`. La langue vient de la réponse.
- Les blocs de moins d'une seconde (fin de réunion, par exemple) ne sont pas envoyés.
- `context_bias` : les termes du vocabulaire de l'utilisateur, 100 au plus. Mistral annonce un support expérimental hors anglais ; la correction locale (`vocabulary::correct`) s'applique ensuite comme aujourd'hui.
- Découpage : morceaux d'environ 10 minutes, coupés au passage le plus calme des dernières secondes. La fonction `blocks` de `src-tauri/src/sched.rs` passe dans `crates/core/src/audio.rs` avec la durée en paramètre ; `MeetingTranscriber` (30 s) et `MistralTranscriber` (600 s) l'utilisent tous deux. Les horodatages de chaque morceau sont décalés de son début.
- Le `MistralTranscriber` n'est pas enveloppé dans `MeetingTranscriber` : il n'occupe pas le processeur, il ne prend donc pas le verrou qui donne la priorité à la dictée.
- `confidence` vaut 1.0 (Mistral n'en fournit pas). Conséquence : pas de zone « confiance faible » à la relecture pour ces réunions ; les zones « chevauchement » restent.
- `NetPolicy::check` est appelé avant chaque requête : une réunion en mode `local` ne peut pas partir chez Mistral.
- `processing.providers.transcription` vaut `mistral-voxtral`.

## Erreurs

La réunion passe en `failed` avec un code, affiché par l'interface (i18n) :
- `mistral_key_missing` : pas de clé dans le trousseau ;
- `mistral_auth` : réponse 401 ou 403 ;
- `mistral_quota` : réponse 429 ;
- `mistral_unreachable` : pas de réseau ou délai dépassé ;
- `mistral_failed` : autre réponse en erreur (le code HTTP va dans les journaux, jamais le contenu).

Le bouton « Réessayer » existant relance le traitement ; l'audio reste sur le disque. Les étapes déjà faites ne sont pas refaites (persistance par étape du pipeline).

## Tests

- `blocks` déplacé : ses tests suivent, plus un cas à 600 s.
- Lecture d'une réponse Mistral enregistrée (JSON fixe) : segments, décalage des horodatages, langue.
- Construction de la requête : champs multipart attendus, vocabulaire tronqué à 100.
- Codes d'erreur selon le statut HTTP.
- Le test de l'invariant 3 (pas de réseau en mode `local`) couvre le nouveau client.
- Test manuel de Jon : une nouvelle réunion avec sa clé (une réunion déjà transcrite garde son étape de transcription).

## À vérifier pendant le plan

- Multipart avec `ureq` 3.4.2 (fonction intégrée ou corps construit à la main).
- Taille et durée maximales acceptées par Mistral par requête ; ajuster les 10 minutes si besoin.
- Format exact de la réponse avec `timestamp_granularities=segment` (champ `segments`, `language`).
- Mise à jour de la spec section 6 (mode `api` : Mistral au lieu de Deepgram) et de `decisions.md`.
