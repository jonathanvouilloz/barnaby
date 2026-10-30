# SPEC — Barnaby : transcription vocale et assistant de réunion, local-first

> Statut : brouillon v0.3
> Licence : MIT (code). Politique de marque à part, voir section 20.
> Langue du document : français. Le code, les identifiants et l'API sont en anglais.
> Référence d'architecture : [Handy](https://github.com/cjpais/Handy) (MIT, Tauri 2 + Rust).

---

## 1. Vision

Barnaby est une application de bureau en deux modes, partageant le même moteur local :

1. **Mode Dictée** : tu maintiens un raccourci, tu parles, et le texte transcrit est collé à la position du curseur, dans n'importe quelle application. Comme Handy.
2. **Mode Réunion** : tu enregistres un appel (quelle que soit la plateforme), Barnaby identifie les locuteurs, produit un transcript structuré, puis te laisse corriger les zones d'ombre avant envoi vers ton système.

Principes communs :

- **Local d'abord** : par défaut, aucune donnée ne quitte la machine.
- **Agnostique de la plateforme** : on capture l'audio du système, pas une API de Meet, Teams, Zoom, WhatsApp ou Discord.
- **Humain dans la boucle** : le système signale ce qu'il ne sait pas, et l'utilisateur tranche.

## 2. Non-objectifs (v1)

- Pas de bot qui rejoint l'appel.
- Pas de multi-utilisateurs, de comptes ou de rôles.
- Pas de traitement en temps réel obligatoire pour le mode Réunion.
- Pas de support mobile.
- Pas d'intégration calendrier autre que Google (Microsoft 365 hors v1).
- Pas de dictée vers une application mobile ou distante.

## 3. Utilisateurs et cas d'usage

| Profil | Machine | Mode de traitement |
|---|---|---|
| Freelance | Ordinaire ou bon PC | Local ou API |
| Utilisateur exigeant | Quelconque | API (qualité maximale) |
| Entreprise (usage interne) | Serveur + postes | Local, configuration centralisée (hors v1) |

Cas d'usage :

1. **Dictée** : écrire un mail, une note ou du code en parlant, avec le texte collé là où est le curseur.
2. **Réunion** : démarrer un enregistrement d'appel avec un raccourci, puis l'arrêter.
3. **Notes pendant l'appel** : fenêtre flottante translucide, par-dessus le client d'appel.
4. **Relecture** : recevoir une alerte à la fin du traitement, relire les zones d'ombre, corriger.
5. **Envoi** : transmettre le résultat final vers un webhook ou un dossier local.

## 4. Plateformes

- **macOS** 13 et plus (Apple Silicon prioritaire, Intel best-effort).
- **Windows** 10 et plus (x64).
- **Linux** : Ubuntu 22.04+ et distributions équivalentes, X11 et Wayland. Limitations documentées (section 17).

## 5. Architecture

```
┌────────────────────────────────────────────────────────┐
│  Barnaby (Tauri 2, Rust + frontend TypeScript)          │
│  ├─ UI : design Apple-like, clair/sombre                │
│  ├─ Raccourcis globaux + CLI (single instance)          │
│  ├─ Capture micro (dictée et réunion)                   │
│  ├─ Capture audio système (réunion uniquement)          │
│  ├─ Mode Dictée : VAD → transcription → injection       │
│  ├─ Mode Réunion : VAD → transcription → diarisation    │
│  │   → identification → structuration                   │
│  ├─ Fenêtre de notes (overlay translucide)              │
│  ├─ Gestionnaire de modèles locaux                      │
│  ├─ SQLite local (réunions, notes, dictées, corrections)│
│  └─ Intégrations sortantes (webhook, dossier)           │
└───────────────┬────────────────────────────────────────┘
                │ providers (trait commun)
     ┌──────────┴──────────────┬────────────────────┐
     │ local                   │ api                │
     │ whisper.cpp / Parakeet  │ Deepgram (STT)     │
     │ sherpa-onnx (diarisation)│ OpenRouter (LLM)  │
     │ Ollama (LLM)            │                    │
     └─────────────────────────┴────────────────────┘
```

Choix techniques :

- **Tauri 2 + Rust** : même base que Handy. Pas de Python embarqué.
- **Transcription locale** : whisper.cpp (GGML) et Parakeet, chargés dans le processus. Accélération GPU quand disponible.
- **VAD** : Silero, pour ignorer les silences.
- **Diarisation locale** : sherpa-onnx (voir section 19).
- **LLM** : un seul client compatible OpenAI, pointant vers Ollama en local ou OpenRouter en API.
- **Frontend** : TypeScript + Vite, tests Playwright, Bun.
- **Stockage** : SQLite. Schémas JSON versionnés pour les échanges.

## 6. Modes de traitement

Chaque étape passe par un trait `Provider` commun (`transcribe`, `diarize`, `identify`, `structure`).

| Mode | Transcription | Diarisation | Identification | Structuration |
|---|---|---|---|---|
| `local` | whisper.cpp ou Parakeet | sherpa-onnx | embeddings locaux | Ollama |
| `api` | Mistral Voxtral (réunions) | sherpa-onnx | embeddings locaux | OpenRouter (modèle au choix) |
| `hybrid` | local | sherpa-onnx | local | OpenRouter (texte uniquement) |

Le mode Dictée utilise toujours la transcription du mode choisi, mais **sans diarisation ni structuration**.

Règles :

- En mode `local`, **aucun appel réseau pendant le traitement**. Le seul téléchargement autorisé est celui d'un modèle, déclenché par l'utilisateur. Cette règle est testée (section 16).
- En mode `api`, l'application affiche quelles données partent, chez quel fournisseur, avant la première utilisation.
- Les clés API sont dans le trousseau de l'OS, jamais en clair.
- **OpenRouter** est la passerelle LLM. L'utilisateur choisit le modèle dans la liste (section 19).
- Les modèles locaux se téléchargent depuis Hugging Face à la demande, ou se placent manuellement dans `models/`. Un redémarrage est nécessaire après un ajout manuel (comportement de Handy).

## 7. Mode Dictée

### 7.1 Comportement

- **Mode au choix dans les réglages** :
  - **Maintien** : enregistre tant que la touche est enfoncée, relâcher arrête et transcrit.
  - **Bascule** : un appui démarre, un second appui arrête et transcrit.
- Le texte transcrit est **collé à la position du curseur** dans l'application active.
- Si aucun champ de texte n'est actif, rien n'est collé et le texte est copié dans l'historique.
- L'indicateur de la barre de menus s'allume pendant l'enregistrement.

### 7.2 Injection du texte

Repris de Handy :

- Méthode principale : copie dans le presse-papiers, simulation du raccourci de collage, puis restauration du contenu précédent du presse-papiers après un délai.
- Délais réglables (avant et après collage).
- Mode « collage fiable » (macOS et Windows) : s'appuie sur les notifications de lecture du presse-papiers, pour éviter les collages ratés.
- Linux : `xdotool` (X11), `wtype` ou `dotool` (Wayland). Sans ces outils, repli sur `enigo` avec compatibilité limitée sous Wayland.
- macOS : permissions Accessibilité et Microphone requises.

### 7.3 Exigences

- Latence entre la fin de la parole et le collage : inférieure à 1 seconde avec un modèle petit ou turbo sur GPU, à mesurer sur CPU.
- **Aucune capture audio système** en mode Dictée. Seul le micro est utilisé.
- Par défaut, l'audio de dictée est supprimé immédiatement après transcription. L'historique garde le texte seulement, avec option de suppression.
- Un modèle dédié peut être choisi séparément de celui du mode Réunion (par exemple un modèle plus léger pour la dictée).

### 7.4 Conflit entre les deux modes

- Si une réunion est en cours, la dictée reste disponible, mais le micro est partagé. L'utilisateur est averti et peut choisir de suspendre l'enregistrement de réunion.
- Pas de dictée pendant le traitement d'une réunion sur le même modèle : le traitement est mis en file, la dictée passe devant.

## 8. Pipeline et états d'une réunion

États :

```
recording → captured → queued → processing → needs_review → reviewed → sending → sent
                                     │                                     │
                                     └──────────► failed ◄─────────────────┘
```

- `recording` : capture en cours.
- `captured` : fichier audio fermé sur disque.
- `queued` : en attente de traitement.
- `processing` : étapes en cours.
- `needs_review` : traitement fini, zones d'ombre détectées.
- `reviewed` : l'utilisateur a validé.
- `sending` / `sent` : envoi vers la destination configurée.
- `failed` : erreur avec cause, relance possible.

Règles :

- L'audio est écrit sur disque pendant l'enregistrement. Un crash ne doit jamais perdre la réunion.
- Chaque étape est idempotente : relancer ne produit pas de doublons.
- Un traitement interrompu reprend à l'étape échouée au redémarrage.

## 9. Capture audio (mode Réunion)

- Capture simultanée du **micro** et de l'**audio système**.
- Les deux pistes sont enregistrées séparément, puis mixées pour la transcription. La piste micro identifie l'utilisateur.
- Format : WAV 16 kHz mono par piste, ou FLAC pour réduire la taille.

La piste système mélange toutes les sorties audio actives (haut-parleurs, casque, sortie mains libres Bluetooth), pas seulement la sortie par défaut : une application d'appel joue souvent ailleurs. Les sorties sont recherchées toutes les 3 s pendant l'enregistrement. Réglages › Réunion permet d'exclure une sortie.

Par système d'exploitation :

- **Windows** : loopback WASAPI pour l'audio système, une capture par sortie active.
- **macOS** : ScreenCaptureKit (macOS 13+), avec permission « Enregistrement de l'écran et audio ». Un driver virtuel (BlackHole) est une solution de repli.
- **Linux** : moniteur PipeWire ou PulseAudio (source `.monitor` de chaque sortie).

Erreurs à afficher clairement :

- Permission refusée.
- Aucun périphérique détecté.
- Piste système silencieuse plus de 30 secondes.

## 10. Interface utilisateur

### 10.1 Principes de design

- Style Apple-like : clarté, espacement généreux, typographie système, coins arrondis, matériaux translucides là où ça a du sens.
- Mode clair et sombre, suivant le système.
- Une action principale par écran.
- Animations sobres, jamais bloquantes.
- Accessibilité : contraste WCAG AA, navigation clavier complète, compatibilité lecteur d'écran.
- Langue : français par défaut, anglais disponible.

### 10.2 Raccourcis (par défaut, personnalisables)

| Action | macOS | Windows / Linux |
|---|---|---|
| Dictée (maintien ou bascule, selon réglage) | ⌥Espace | Ctrl+Alt+Espace |
| Démarrer / arrêter la réunion | ⌘⇧R | Ctrl+Shift+R |
| Afficher / masquer la fenêtre de notes | ⌘⇧N | Ctrl+Shift+N |
| Marqueur « point important » | ⌘⇧M | Ctrl+Shift+M |
| Ouvrir la file de relecture | ⌘⇧L | Ctrl+Shift+L |

Tous les raccourcis sont configurables dans les réglages, avec détection des conflits avec le système et les autres applications. Le mode de la dictée (maintien ou bascule) se règle au même endroit.

Pilotage en ligne de commande d'une instance déjà lancée (`barnaby --toggle`, `--cancel`), utile pour les intégrations.

### 10.3 Fenêtre de notes (overlay)

- Fenêtre sans bordure, translucide (opacité de 40 à 100 %), toujours au-dessus.
- Déplaçable, redimensionnable, mémorise sa position.
- Saisie simple, horodatage automatique de chaque note.
- Reste visible par-dessus Google Meet, Teams ou WhatsApp Web.
- Limitation Wayland : dépend du compositeur. Mode de repli : fenêtre normale.

### 10.4 Mascotte : la chouette

- Petite chouette animée, discrète, dans la barre de menu ou un coin de l'écran.
- Réagit à l'état du système, jamais au contenu de la conversation :
  - au repos : posée,
  - dictée ou enregistrement : à l'écoute,
  - traitement : réflexion,
  - besoin de relecture : tête penchée,
  - envoi réussi : brève animation,
  - erreur : alerte.
- Désactivable en un clic. SVG animé, léger.
- Le logo et la mascotte sont créés par l'auteur (voir section 20).

### 10.5 Écrans

1. **Accueil** : dictées récentes et réunions, avec statut.
2. **Enregistrement** : titre, participants, indicateur de niveau audio.
3. **Relecture** : transcript à gauche, zones d'ombre à droite, lecture audio synchronisée.
4. **Réglages** : mode de traitement, modèles (dictée et réunion séparément), clés API, destinations, raccourcis, rétention, langue.

## 11. Participants

Phase 1 : saisie manuelle en début de réunion. L’utilisateur y figure d’office, sous le prénom donné à l’onboarding ou dans Réglages (« Moi » sinon), et n’a pas à s’ajouter.

Phase 2 : **Google Calendar uniquement** (v1), en lecture seule via OAuth. Toujours validé par l'utilisateur.

Les participants saisis (sans l’utilisateur) fixent le nombre de voix que cherche la diarisation (leur nombre + 1, à partir d’une minute d’audio) et servent à proposer des noms aux locuteurs, pas à les attribuer de façon définitive.

## 12. Identification des locuteurs

- **Diarisation** : qui parle quand (sherpa-onnx en local, Deepgram en API).
- **Identification** : rattachement d'un locuteur à un participant, par enrôlement vocal ou par déduction (piste micro = utilisateur).
- **Enrôlement vocal** : optionnel et consenti. Une empreinte vocale est une donnée biométrique : chiffrée, supprimable, jamais envoyée à un tiers en mode `local`.
- Sans enrôlement : « Speaker A », « Speaker B », etc., renommés à la relecture.

## 13. Zones d'ombre et relecture

Une zone d'ombre est créée quand :

- la confiance de transcription est sous 0,6 (réglable),
- deux locuteurs se chevauchent plus d'une seconde,
- un changement de locuteur est détecté mais non attribué,
- un nom propre ou un terme technique est incertain,
- une portion est inaudible.

Chaque zone contient : horodatage, extrait audio, texte proposé, alternatives, question formulée.

Actions : valider, corriger, réattribuer le locuteur, marquer « inaudible », ignorer.

Les corrections de noms sont réutilisées pour les réunions suivantes avec les mêmes personnes.

**Alerte de fin de traitement** : notification système et badge. Pas de son par défaut, son optionnel. Jamais d'affichage intrusif pendant un appel.

## 14. Format de sortie

Réunion (JSON versionné) :

```json
{
  "schema_version": "1.0",
  "meeting": {
    "id": "uuid",
    "title": "string",
    "started_at": "ISO-8601",
    "ended_at": "ISO-8601",
    "language": "fr",
    "participants": [
      { "id": "p1", "name": "string", "email": "string|null", "confirmed": true }
    ]
  },
  "segments": [
    {
      "id": "s1",
      "start": 12.4,
      "end": 16.1,
      "speaker_id": "p1",
      "text": "string",
      "confidence": 0.93,
      "reviewed": true
    }
  ],
  "notes": [
    { "id": "n1", "at": 320.0, "text": "string", "source": "manual" }
  ],
  "summary": {
    "overview": "string",
    "decisions": ["string"],
    "action_items": [
      { "owner": "p1", "task": "string", "due": "string|null" }
    ],
    "open_questions": ["string"]
  },
  "processing": {
    "mode": "local|api|hybrid",
    "providers": { "transcription": "whisper.cpp", "diarization": "sherpa-onnx" },
    "review_items_total": 7,
    "review_items_resolved": 7
  }
}
```

Dictée (JSON versionné, plus léger) :

```json
{
  "schema_version": "1.0",
  "dictation": {
    "id": "uuid",
    "created_at": "ISO-8601",
    "text": "string",
    "language": "fr",
    "duration_ms": 4200,
    "injected_into": "app name|null",
    "provider": "whisper.cpp"
  }
}
```

## 15. Intégrations sortantes

Chaque destination implémente `send(meeting_json) → result`. Les dictées ne sont pas envoyées par défaut.

L'envoi part **dès que la relecture est validée**, sans étape supplémentaire.

Fournies en v1 :

- **Webhook HTTP** : POST JSON, en-tête `X-Signature: sha256=<HMAC>`. Réessais exponentiels (3 à 5 tentatives). Idempotence via `meeting.id`.
- **Dossier local** : `<meeting_id>.json` et `<meeting_id>.md`.

Hors v1 : Notion, Google Drive, Obsidian, e-mail, base de données.

Un envoi échoué ne fait jamais perdre la réunion. Elle reste en `failed` avec « réessayer ».

## 16. Données, confidentialité et conformité

- **Stockage** : répertoire utilisateur, chiffrement au repos optionnel.
- **Rétention par défaut, réunion** : audio supprimé à la validation de la relecture. Options : 7 jours, ou illimitée. Le transcript n'est jamais supprimé automatiquement.
- **Rétention par défaut, dictée** : audio supprimé immédiatement après transcription. Texte conservé dans l'historique, suppression manuelle possible.
- **Suppression** : une réunion peut être supprimée en totalité, y compris les empreintes vocales liées.
- **Consentement** : rappel avant le premier enregistrement de réunion, avec texte type pour informer les participants.
- **RGPD** : empreintes vocales soumises à consentement explicite. Le mode `api` mentionne les sous-traitants (Deepgram, OpenRouter, fournisseur du modèle) et leur localisation.
- **Journaux** : aucun contenu, seulement des identifiants et codes d'erreur.
- **Télémétrie** : désactivée par défaut, aucune donnée de contenu.

Tests de sécurité :

- Test automatisé : aucune requête réseau en mode `local` pendant le traitement, réunion ou dictée.
- Test de la signature HMAC du webhook.

## 17. Exigences non fonctionnelles

Performance (référence : PC 16 Go de RAM, GPU 8 Go ou CPU récent) :

- Dictée : collage en moins de 1 seconde après la fin de la parole, avec modèle petit ou turbo sur GPU.
- Réunion : traitement local inférieur à 1× la durée de l'appel avec GPU. Sur CPU, à mesurer en Phase 0.
- Démarrage de l'application : moins de 3 secondes.
- Taille de l'application sans modèles : moins de 50 Mo.

Fiabilité :

- Aucune perte d'audio en cas de crash.
- Reprise automatique du traitement au redémarrage.

Limitations connues :

- Overlay Wayland : dépend du compositeur.
- Injection de texte sous Wayland : limitée, comme dans Handy.
- Capture système macOS : permission requise, macOS 13 minimum.
- Diarisation dégradée en cas de chevauchement important.

## 18. Évaluation qualité

Métriques :

- **DER** pour la diarisation.
- **WER** pour la transcription, en français et en anglais, pour la dictée et la réunion.
- **Zones d'ombre** par heure d'audio.
- **Corrections** à la relecture.

Jeu de test :

- 10 réunions de 10 à 30 minutes, annotées à la main, anonymisées.
- Pour la dictée : 30 minutes de phrases courtes en français et en anglais, avec ponctuation et noms propres.
- Benchmarks publics AMI ou CALLHOME pour comparer sherpa-onnx et pyannote, avec le même modèle d'embeddings.
- Non-régression : une modification ne doit pas dégrader une métrique de plus de 5 %.

## 19. Décisions de fond (deep dive)

Les trois questions ouvertes de la v0.2 ont été approfondies. Les sources sont en bas de section, et les chiffres doivent être revérifiés avant implémentation.

### 19.1 Diarisation locale : sherpa-onnx ou pyannote ?

Constat :

- sherpa-onnx embarque le **même réseau de segmentation que pyannote**, exporté en ONNX, sous licence MIT d'origine. Il ajoute un modèle d'embeddings WeSpeaker.
- Aucune comparaison DER directe n'a été trouvée entre les deux. Un écart éventuel viendrait donc surtout de l'embedding et du clustering, pas de la segmentation.
- sherpa-onnx ne publie pas de DER, et n'attribue qu'un seul locuteur par segment : le chevauchement n'est pas étiqueté. Ce point compte pour nos zones d'ombre.
- Les chiffres de DER de pyannote trouvés sur des blogs (environ 11 à 19 % sur AMI et CALLHOME) n'ont pas de méthodologie claire. À considérer comme indicatifs.

**Décision :** sherpa-onnx pour le mode `local`. Il s'intègre dans le binaire Rust, sans Python, ce qui colle à l'architecture de Handy. Le test de Phase 0 compare les deux sur AMI ou CALLHOME. Si l'écart est important, pyannote devient une option derrière un sidecar Python, sans changer le contrat du `Provider`.

### 19.2 OpenRouter : quel modèle par défaut ?

Constat :

- Les prix trouvés varient fortement selon les sites, parfois d'un facteur 10 pour le même modèle. Il faut lire `/api/v1/models` en direct.
- OpenRouter prend environ 5,5 % (minimum 0,80 $) sur les crédits achetés, selon un site tiers.
- Aucune source trouvée ne confirme quels modèles supportent le **JSON structuré** (schéma imposé). C'est pourtant le critère critique pour la structuration.

**Décision :**
- Pas de modèle figé dans la spec. Le défaut est choisi en Phase 0, sur le corpus d'évaluation.
- Critères de choix, dans l'ordre : respect du schéma JSON (100 % sur le corpus de test), qualité du français, coût par réunion d'une heure.
- Le mode `api` affiche le coût estimé avant chaque traitement.
- Filtre à appliquer : ne retenir que les modèles dont `supported_parameters` contient `response_format` ou `structured_outputs`.

### 19.3 Deepgram : transcription API

Constat :

- Nova-3 Multilingual en streaming : tarifs annoncés entre 0,0058 et 0,0092 $ par minute, selon promotion et source.
- Pré-enregistré (batch) : environ 0,0043 à 0,0052 $ par minute.
- Diarisation : environ 0,002 $ par minute de plus.
- Fourchette totale pour une heure de réunion avec diarisation : environ 0,50 à 0,75 $. Tarifs à confirmer sur deepgram.com avant budget.
- Nova-3 gère le changement de langue en cours de phrase sur 10 langues dont le français. Pas de benchmark français trouvé.

**Décision :** Deepgram en mode batch pour les réunions (on traite après l'appel, pas besoin du streaming). Le streaming n'est utile que pour une dictée API, reportée hors v1.

### 19.4 Logo, mascotte et licence

Constat :

- Le dépôt de Handy indique que le code est sous MIT, mais que le nom, le logo et les éléments de marque ne sont pas open source, et que les forks doivent avoir leur propre identité visuelle. Cette information vient d'un résumé de la page GitHub, à vérifier dans le dépôt.
- La recherche n'a pas trouvé de politique de marque plus détaillée.

**Décision proposée :**
- **Code** : MIT.
- **Logo et mascotte** : sous une licence séparée, par exemple CC BY 4.0 (réutilisation avec attribution). Plus simple que la marque.
- **Nom « Barnaby »** : fichier `TRADEMARK.md` qui demande aux forks de changer de nom s'ils distribuent une version modifiée. Ça évite la confusion sans bloquer le code.

Sources des sections 19.1 à 19.4 :
- [sherpa-onnx offline speaker diarization (exemples)](https://github.com/k2-fsa/sherpa-onnx/blob/master/python-api-examples/offline-speaker-diarization.py)
- [Speaker diarization comparison (2026)](https://brasstranscripts.com/blog/speaker-diarization-models-comparison)
- [OpenRouter pricing (agrégat)](https://pricepertoken.com/endpoints/openrouter)
- [OpenRouter fees (ofox)](https://ofox.ai/blog/openrouter-pricing-hidden-markup-breakdown-2026/)
- [Deepgram Nova-3 pricing (tiers)](https://convertaudiototext.com/blog/deepgram-nova-3-explained)
- [Handy (dépôt GitHub)](https://github.com/cjpais/Handy)

## 20. Licence et gouvernance

- **Code** : MIT.
- **Logo et mascotte** : CC BY 4.0 (voir 19.4).
- **Nom** : politique de marque dans `TRADEMARK.md`.
- Contributions : `CONTRIBUTING.md`, décisions d'architecture dans `docs/adr/`.
- `CODE_OF_CONDUCT.md` standard.

## 21. Roadmap

**Phase 0 : preuve de concept (2 à 3 semaines)**
- Script CLI : enregistrement, transcription (whisper.cpp), diarisation (sherpa-onnx), JSON.
- Comparer sherpa-onnx et pyannote sur AMI ou CALLHOME, et sur 3 à 5 réunions réelles anonymisées.
- Choisir le modèle OpenRouter par défaut selon 19.2.
- Valider Deepgram batch et son tarif réel.

**Phase 1 : MVP local (4 à 6 semaines)**
- App Tauri : mode Dictée complet sur un OS.
- Mode Réunion : capture, traitement local, relecture simple.
- Saisie manuelle des participants.
- Envoi vers dossier local et webhook.

**Phase 2 : multiplateforme, notes et API**
- Captures et injection validées sur Windows et macOS.
- Fenêtre de notes, raccourcis, mascotte.
- Mode `api` : Deepgram et OpenRouter.

**Phase 3 : finitions**
- Google Calendar.
- Enrôlement vocal.
- Mode `hybrid`.
- Packaging et signature (macOS notarisé, Windows signé).

## 22. Décisions tranchées (v0.3)

| Sujet | Décision |
|---|---|
| Nom | Barnaby |
| Mascotte | Une chouette, créée par l'auteur |
| Licence code | MIT |
| Licence logo | CC BY 4.0 |
| Marque | `TRADEMARK.md`, forks renommés |
| Architecture | Tauri 2 + Rust, sans Python embarqué |
| Modes | Dictée et Réunion, moteur partagé |
| Dictée | Maintien ou bascule (réglage), collage au curseur, audio supprimé immédiatement |
| Raccourcis | Tous configurables dans les réglages |
| Envoi | Dès validation de la relecture |
| Diarisation locale | sherpa-onnx, pyannote en repli si la Phase 0 le justifie |
| API LLM | OpenRouter, modèle choisi par l'utilisateur, défaut tranché en Phase 0 |
| API transcription | Deepgram batch pour les réunions |
| LLM local | Ollama |
| Langues | Interface FR (EN disponible), transcription FR/EN avec détection |
| Calendrier | Google uniquement en v1 |
| Notification de fin | Système, son optionnel (désactivé par défaut) |
| Rétention réunion | Audio supprimé à la validation, option 7 jours |
| Rétention dictée | Audio supprimé immédiatement |

## 23. Questions ouvertes

Aucune question bloquante. Les points restants sont des choix d'implémentation, traités en Phase 0 ou Phase 1.
