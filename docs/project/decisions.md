# Décisions — Barnaby

Les décisions produit (sections 19 et 22) sont dans `docs/SPEC.md`. Ici, seulement les choix d'implémentation.

## 2026-10-21 — Frontend : Svelte 5 + Vite, sans SvelteKit
- Choix : Svelte 5 (runes) + Vite + TypeScript, construit avec Bun, dans la webview Tauri.
- Écarté : React (comme Handy : runtime plus lourd, moins familier pour le propriétaire), TS vanilla (état et composants réécrits à la main), SvelteKit (son routeur et son SSR ne servent à rien dans une app de bureau).
- Pourquoi : runtime quasi nul (objectif app < 50 Mo, démarrage < 3 s), stack principale du propriétaire, qui a demandé « la plus efficiente ».
- À rouvrir si : on veut réutiliser des composants de Handy tels quels.

## 2026-10-21 — Phase 1 : Windows et Linux, sans passer par la Phase 0
- Choix : MVP local (spec section 21, Phase 1) sur Windows x64 et Linux. macOS compile mais n'est pas validé.
- Pourquoi : le propriétaire teste sur Windows et Linux. Les mesures de la Phase 0 (DER, WER) se font sur son PC, avec le corpus réel.
- À rouvrir si : il faut une version macOS.

## 2026-10-21 — Workspace en trois crates Rust
- Choix : `barnaby-core` (logique pure, testée partout), `barnaby-engine` (audio, whisper, sherpa : dépendances natives), `src-tauri` (app).
- Pourquoi : le VPS de développement n'a ni ALSA ni webkit2gtk. Le cœur se teste partout, le natif se compile en CI (GitHub Actions, Windows + Ubuntu).
- À rouvrir si : la frontière entre les crates finit par coûter plus d'indirection qu'elle ne fait gagner.

## 2026-10-21 — Audio système Linux via `parec`
- Choix : sous Linux, la piste système est lue avec `parec --device=@DEFAULT_MONITOR@` (pipewire-pulse ou PulseAudio) ; sous Windows, avec un loopback WASAPI via cpal.
- Écarté : bindings libpulse/libpipewire (une dépendance native de plus pour lire un flux PCM).
- Pourquoi : `parec` est présent sur Ubuntu 22.04+ (pipewire-pulse) et fonctionne avec les deux serveurs.
- À rouvrir si : `parec` manque sur une distribution cible.

## 2026-10-21 — Design : brief « Apple-like » de la spec, sans phase de références
- Choix : `DESIGN.md` minimal, avec la police système et des tokens clair/sombre, tirés de la spec (section 10.1).
- Pourquoi : le propriétaire a voulu avancer sur le build. La spec fixe déjà la direction et la typographie système.
- À rouvrir si : une refonte visuelle est demandée (passer alors par `design-references` puis `/impeccable`).

## 2026-10-21 — Dictée : Parakeet v3 en flux plutôt que Whisper
- Choix : Parakeet TDT 0.6b v3 int8 via sherpa-onnx (déjà embarqué). Chaque segment de parole est transcrit dès sa fin (VAD Silero), le modèle est préchargé au démarrage. Whisper reste réservé aux réunions.
- Écarté : whisper en fin de dictée (6 s pour 5 s de parole sur CPU, à cause des fenêtres de 30 s) ; transcription cloud comme Wispr Flow ou Willow (l'audio quitterait la machine).
- Pourquoi : 0,47 s entre l'arrêt et le texte pour 62 s de dictée, mesuré sur 2 vCPU. Texte français identique à whisper et ponctué. Objectif du propriétaire : environ 0,3 s, comme Willow.
- À rouvrir si : la latence ressentie sur le PC du propriétaire dépasse 0,5 s, ou si la qualité française décroche sur ses dictées. Revue après son test.

## 2026-10-21 — Réunions : traitement après l'appel, résumé local ou cloud
- Choix : on enregistre, puis on traite à la fin (whisper + sherpa en local). Le résumé passe par Ollama (`local`) ou OpenRouter (`hybrid`, clé dans le trousseau, avertissement sur les données avant la première activation). Seul le texte part.
- Écarté : transcrire les réunions en direct (la diarisation est meilleure sur l'audio complet, et le délai compte peu) ; envoyer l'audio au cloud.
- À rouvrir si : le traitement local d'une réunion d'une heure dépasse la durée de l'appel sur le PC du propriétaire. Piste : build GPU Vulkan.

## 2026-10-21 — Vocabulaire : correction approchée locale, plus prompt LLM
- Choix : distance d'édition sur une forme normalisée, au plus 1 ou 2 éditions selon la longueur, première lettre identique, liste de mots courants exclus. Les termes sont aussi donnés au LLM du résumé.
- Écarté : hotwords de sherpa (non garantis pour le transducer NeMo) ; correction par LLM de chaque dictée (latence, réseau).
- À rouvrir si : les sur-corrections gênent en pratique (exemple connu : « barbier concept »).

## 2026-10-21 — Overlay Windows : déplacé hors écran, jamais caché
- Choix : la fenêtre est créée visible et non focalisable, puis on la déplace à l'écran ou hors écran. Le premier déplacement hors écran se fait juste après la création : tao ignore une position de départ située hors de tout écran et pose la fenêtre en plein écran (bug vu au premier test).
- Pourquoi : `ShowWindow` peut l'activer malgré `WS_EX_NOACTIVATE`, et le collage partirait alors dans l'overlay.
- À rouvrir si : le test sur le PC du propriétaire montre un vol de focus ou un artefact visuel.

## 2026-10-21 — CI : pas de cache Rust au build, pas d'AppImage
- Choix : le job de build compile sans rust-cache ; Linux ne livre que le `.deb`.
- Pourquoi : le cache restaurait sherpa-rs-sys sans ses bibliothèques partagées, d'où des tests et un bundle cassés. `linuxdeploy` échoue sur l'AppImage.
- À rouvrir si : la durée de la CI devient gênante (piste : copier les bibliothèques depuis `~/.cache/sherpa-rs` après la restauration), ou si une distribution non Debian est demandée.

## 2026-10-21 — Mascotte : vidéo générée par IA, décorative
- Choix : chouette « esprit » 2D minimaliste, générée en image puis animée en image vers vidéo (Kling) : boucle de 5 s, carré 1:1. Usages : le site (WebM + MP4, 800 px) et l'onboarding de l'app (WebM, 512 px). Dans l'app, elle est décorative.
- Écarté : animation SVG ou Rive pilotée par l'état (jugée trop coûteuse pour le besoin) ; Kling pour une animation qui réagit en direct (une vidéo ne change pas d'état).
- Ouvert : retirer de la spec (10.4) et du code les six états de la chouette. Le propriétaire n'a pas encore tranché.
- À rouvrir si : une boucle vidéo pèse trop ou rend mal sur fond clair et sombre.

## 2026-10-22 — Son système : toutes les sorties, mélangées
- Choix : la piste système capture chaque sortie active (loopback WASAPI par sortie, un `parec` par moniteur sous Linux), rescannées toutes les 3 s, et les mélange sur l'horloge. L'utilisateur peut exclure une sortie.
- Pourquoi : au premier test Windows, Google Meet n'était pas enregistré alors que YouTube l'était. Seule la sortie par défaut était écoutée, et Meet joue souvent sur la sortie de communication ou mains libres.
- Écarté : la capture par processus de Windows (build 20348 ou plus, API bas niveau absente de cpal) ; la documentation ne dit pas si elle couvre toutes les sorties.
- À rouvrir si : un cas réel échoue encore (plan B : capture par processus), ou si un son joué sur une autre sortie gêne la transcription.

## 2026-10-22 — Réunions : transcription Mistral Voxtral en option
- Choix : API Mistral en direct, clé propre (`mistral-api-key`, trousseau), réunions seulement. Le modèle `mistral-voxtral` met `processing_mode` à `api`. La diarisation reste locale (sherpa-onnx). Pas de basculement automatique vers le local : une erreur met la réunion en `failed`, « Réessayer » relance.
- Écarté : OpenRouter et Deepgram (un seul fournisseur) ; la diarisation par Mistral (format non documenté) ; la dictée en ligne (l'audio quitterait la machine à chaque phrase).
- Pourquoi : Whisper small est inutilisable sur les réunions réelles ; Voxtral coûte environ 0,18 $ par heure.
- À rouvrir si : la qualité Mistral déçoit sur les réunions de Jon, ou si une coupure réseau en pleine réunion gêne sans repli automatique.

## 2026-10-22 — Refonte « Things, version chouette » et onboarding
- Choix : interface aérée inspirée de Things 3, palette de la chouette (neutres chauds, encre `#0f0c2f`, bleu étincelle `#0170ec` pour les actions, orange `#fcaa4f` pour l'enregistrement), Valley Sans embarquée, étincelle ✦ comme seul signe de ce qui attend une action, chouette (découpes de la planche) aux moments clés seulement, tutoiement dans l'interface. Onboarding en 6 étapes, chacune passable, remplaçant la fenêtre « Modèles à installer » ; ce qui manque reste signalé (✦ sur Réglages).
- Écarté : directions « carnet à onglets » et « carte du ciel » (sombre par défaut) ; test du son système dans l'onboarding ; vidéo Kling tant qu'elle a un fond blanc et un filigrane ; sélecteur de micro (reporté).
- Pourquoi : Jon jugeait l'ancien look vieux et classique ; il a choisi cette direction, la place de la chouette, le tutoiement et le contenu de l'onboarding.
- À rouvrir si : le rendu dans WebView2 ou WebKitGTK diffère des captures Chromium, ou si l'onboarding bloque des utilisateurs réels.
- Revue : après le test Windows de Jon.

## 2026-10-22 — Texte de consentement construit depuis les réglages
- Choix : le texte à lire aux participants dit la vérité selon le modèle (local ou Mistral, « selon ses conditions »), le résumé en ligne et la durée de conservation réelle ; il reste au vouvoiement et toujours visible sur l'écran « Nouvelle réunion ».
- Pourquoi : la revue d'accessibilité et de consentement a trouvé que l'ancien texte promettait un audio local et supprimé, faux avec Mistral ou une conservation longue.
- À rouvrir si : un nouveau fournisseur ou une nouvelle règle de conservation apparaît.

## 2026-10-22 — Version macOS : Apple Silicon, macOS 14.2+, capture par tap global
- Choix : `.dmg` pour Mac à puce Apple, macOS 14.2 minimum, signé ad hoc (pas de compte développeur Apple). Son de l'ordinateur par un « process tap » global de Core Audio (tous les processus, toutes les sorties) sur un agrégat privé lu par cpal ; sherpa-onnx lié en statique, whisper.cpp sur Metal ; raccourcis ⌘⇧Espace et ⌘⇧R/M/L (macOS 15 refuse un raccourci global avec Option seul, et Ctrl+Option+Espace change la source de saisie) ; droit micro dans les entitlements (runtime renforcé).
- Écarté : la capture par sortie de cpal (elle enregistre le micro d'un casque qui a aussi une entrée, comme les AirPods) ; ScreenCaptureKit (permission d'enregistrement d'écran) ; Intel et macOS 13 (le tap n'existe pas avant 14.2, build universel plus lourd) ; notarisation (compte payant).
- Pourquoi : demande de Jon, sans Mac pour tester : la solution la plus courte qui couvre tous les casques.
- À rouvrir si : le test sur un vrai Mac montre que l'agrégat privé n'est pas visible (repli public déjà codé), si des utilisateurs Intel le demandent, ou si Jon prend un compte développeur Apple.

## 2026-10-22 — Site de présentation en HTML et CSS seuls
- Choix : une page statique dans `site/` (HTML, CSS, sans étape de compilation), hébergée sur Cloudflare sous un sous-domaine de Jon Labs ; Valley Sans reprise du paquet de l'app. Cadrage : `docs/project/site.md`.
- Écarté : Astro (outil plus gros que le besoin) ; SvelteKit statique (JavaScript inutile pour une page sans interaction).
- Pourquoi : une seule page de texte, en français, sans interaction ; le plus léger pour le visiteur et posable n'importe où.
- À rouvrir si : le site passe à plusieurs pages ou à l'anglais (alors Astro).
