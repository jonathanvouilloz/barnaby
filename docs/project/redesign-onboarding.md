# Refonte visuelle et onboarding

Décisions de Jon, 2026-10-22. Références : `design/references/refs.md`. Direction : contrat dans `.impeccable/surfaces/ui-src-app-svelte.md` (« Things, version chouette »). Produit : `PRODUCT.md`.

## 1. Refonte visuelle

Toute la fenêtre principale change d'apparence, sans changer ce qu'elle fait : mêmes écrans, mêmes actions, mêmes données.

- **Police** : Valley Sans, embarquée via le paquet `@fontsource-variable/valley-sans` (aucun chargement réseau). Repli : police système.
- **Couleurs** : celles de la chouette (`refs.md`). Neutres chauds, bleu étincelle pour les actions, orange pour l'enregistrement. Mode sombre sur fond encre. Contraste AA vérifié pour chaque paire texte/fond ; une couleur de la palette qui échoue est assombrie ou éclaircie, et l'écart est noté dans `DESIGN.md`.
- **Structure** : barre latérale translucide avec petites icônes colorées et compteur ✦ sur Relecture, grands titres d'écran gras, listes sans bordures, une action principale par écran.
- **Signature** : l'étincelle ✦ marque ce qui attend une action (réunion à relire, passage douteux, clé manquante) et s'anime brièvement quand c'est réglé. Coupée avec `prefers-reduced-motion`.
- **Chouette** : aux moments clés seulement (onboarding, écrans vides, fin de traitement, erreurs). `Owl.svelte` (SVG provisoire) reprend les couleurs et le trait de la planche, en attendant la chouette détourée de Jon.
- **Textes** : passage au tutoiement dans toute l'interface française. Ton complice, phrases simples. L'anglais suit le même ton.
- L'overlay de dictée et la fenêtre de notes prennent les nouvelles couleurs et la police ; leur forme ne change pas.

## 2. Onboarding

Remplace la fenêtre actuelle « Modèles à installer » (`ModelSetup.svelte`) au premier lancement. Plein écran dans la fenêtre principale, une étape à la fois, avec un indicateur de progression et « Retour ». Chaque étape peut être passée ; ce qui manque reste signalé ensuite (✦) là où ça se règle.

1. **Bienvenue** : la chouette, une phrase sur ce que fait Barnaby (dicter, enregistrer les réunions), et la promesse « rien ne quitte ton ordinateur sans que tu le choisisses ».
2. **Micro** : niveau du micro en direct (« parle, la barre doit bouger »). Si l'accès est refusé : comment l'autoriser dans les réglages du système.
3. **Modèles** : deux choix clairs.
   - « Sur mon ordinateur » : télécharge le modèle de dictée (Parakeet v3), les modèles de séparation des voix et le modèle de réunion recommandé, avec une barre de progression.
   - « En ligne pour les réunions » : la fenêtre Mistral existante (clé, ce qui part, prix). La dictée reste locale, donc Parakeet se télécharge aussi.
4. **Test de dictée** : choix du raccourci (composant `ShortcutInput` existant), puis une zone de texte de test : on garde le curseur dedans, on dicte avec le raccourci, le texte arrive. Bouton « Ça marche » ou « Réessayer ».
5. **Tour des fonctions** : trois courts panneaux illustrés (notes pendant l'appel, relecture des passages douteux, envoi et résumé), avec les raccourcis correspondants.
6. **Fin** : « Tout est prêt », bouton vers l'accueil.

Règles :
- Ne s'affiche qu'une fois : nouveau réglage `onboarding_done` (dans `model.rs` et `api.ts`, `#[serde(default)]`). Les installations existantes avec des modèles déjà présents sont considérées comme faites.
- Relançable depuis Réglages (« Revoir l'accueil »).
- Fermer l'app au milieu : l'onboarding reprend au début au prochain lancement (aucun état à stocker ; les modèles déjà téléchargés sont reconnus).
- Le test de dictée utilise la vraie dictée (raccourci global, collage dans la zone de test de la fenêtre Barnaby).

## Hors périmètre
- Le logo final et la chouette détourée (fournis plus tard par Jon) ; la vidéo Kling n'est utilisée qu'une fois détourée et sans filigrane.
- Le test de capture du son système (écarté par Jon).
- La mise en page des écrans au-delà du nécessaire à la direction.

## Vérification
- Captures de chaque écran en clair et en sombre (mode navigateur avec `mock.ts`), comparées à `refs.md`, puis revue par `impeccable-finish-reviewer` et `review-ui`.
- `bun run check`, `bun test`, tests Rust pour `onboarding_done`.
- Test de Jon sur Windows : premier lancement complet.
