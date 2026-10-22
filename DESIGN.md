---
name: Barnaby
description: Things, version chouette. Une app de bureau aérée où seule attire l'œil ce qui attend une décision.
colors:
  accent: "#0170ec"
  accent-text: "#005fcc"
  on-accent: "#fdfdfd"
  rec: "#fcaa4f"
  on-rec: "#0f0c2f"
  warm: "#b25f00"
  bg: "#fdfdfd"
  surface: "#ffffff"
  sidebar: "rgb(248 235 220 / .82)"
  owl-field: "#f8ebdc"
  text: "#0f0c2f"
  text-2: "#6f5b52"
  border: "rgb(15 12 47 / .09)"
  border-strong: "rgb(15 12 47 / .5)"
  fill: "rgb(159 136 125 / .10)"
  fill-strong: "rgb(159 136 125 / .16)"
  danger: "#b3261e"
  warn: "#9a5b00"
  ok: "#1f7a3a"
  dark-bg: "#0f0c2f"
  dark-surface: "#18143d"
  dark-sidebar: "rgb(34 29 78 / .82)"
  dark-text: "#f8ebdc"
  dark-text-2: "#b9a596"
  dark-border: "rgb(248 235 220 / .12)"
  dark-border-strong: "rgb(248 235 220 / .42)"
  dark-accent: "#4d9bff"
  dark-on-accent: "#0f0c2f"
  dark-warm: "#fcaa4f"
  dark-owl-field: "#decfc5"
  dark-fill: "rgb(248 235 220 / .06)"
  dark-fill-strong: "rgb(248 235 220 / .11)"
  dark-danger: "#ff8a80"
  dark-warn: "#ffc069"
  dark-ok: "#5bd17f"
typography:
  display:
    fontFamily: "Valley Sans Variable, system-ui, sans-serif"
    fontSize: "28px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.01em"
  counter:
    fontFamily: "Valley Sans Variable, system-ui, sans-serif"
    fontSize: "40px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "-0.01em"
    fontFeature: "\"tnum\""
  headline:
    fontFamily: "Valley Sans Variable, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 650
    lineHeight: 1.35
  body:
    fontFamily: "Valley Sans Variable, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Valley Sans Variable, system-ui, sans-serif"
    fontSize: "12.5px"
    fontWeight: 550
    lineHeight: 1.5
rounded:
  control: "8px"
  card: "12px"
  sheet: "16px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  2xl: "32px"
  screen: "40px"
components:
  button:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "4px 12px"
    height: "32px"
  button-hover:
    backgroundColor: "{colors.fill}"
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-accent}"
    rounded: "{rounded.control}"
    padding: "4px 12px"
    height: "32px"
  button-record:
    backgroundColor: "{colors.rec}"
    textColor: "{colors.on-rec}"
    rounded: "{rounded.control}"
  button-small:
    padding: "0 8px"
    height: "28px"
  field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "4px 12px"
    height: "32px"
  nav-item:
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "4px 8px"
    height: "32px"
  nav-item-current:
    backgroundColor: "{colors.fill-strong}"
  card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.card}"
  sheet:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.sheet}"
    padding: "20px 24px"
    width: "480px"
---

# Design System: Barnaby

## Overview

**Creative North Star: "Things, version chouette"**

Une app de bureau calme et aérée, dans la lignée de Things : beaucoup de blanc, des titres d'écran grands et gras, des lignes de liste sans bordure, une barre latérale translucide aux tons crème. Une seule chose attire l'œil sur un écran : ce qui attend une décision, marqué par l'étincelle bleue. Le reste s'efface.

Les neutres sont chauds (encre, crème, taupe), jamais le gris froid d'Apple. La chouette, dessinée à la main, n'apparaît qu'aux moments clés ; partout ailleurs, son étincelle suffit. Le thème clair est le défaut, le sombre suit le système et pose tout sur un fond encre.

Tokens dans `ui/src/app.css` ; les valeurs du frontmatter en sont la copie, le CSS fait foi.

**Key Characteristics:**
- Contenu blanc, navigation translucide crème, texte encre.
- Un bleu pour l'action et la sélection, un orange pour l'enregistrement.
- Une police, Valley Sans, embarquée.
- L'étincelle comme signature de ce qui attend.
- Plat ; les ombres sont réservées à ce qui flotte.

## Colors

Palette tirée de la planche de la chouette : neutres chauds, deux accents qui ne se croisent pas.

### Primary
- **Bleu étincelle** (accent ; sombre : dark-accent) : bouton principal, sélection, anneau de focus, piste de lecture, radios et interrupteurs cochés, l'étincelle (`--spark`, même valeur).
- **Bleu texte** (accent-text) : tout texte bleu (« À relire », horodatage survolé). Le bleu étincelle échoue AA en texte sur les fonds taupe ; en sombre les deux valeurs se confondent.

### Secondary
- **Orange bec** (rec) : enregistrement seulement (point de la barre latérale, point pulsé, bouton Arrêter). Le texte posé dessus est toujours encre (on-rec) dans les deux thèmes : le crème y échoue.
- **Orange cuit** (warm ; sombre : dark-warm) : l'icône Accueil, assombrie pour tenir 3:1 sur la ligne sélectionnée.

### Neutral
- **Blanc papier** (bg) et **Blanc** (surface) : fond du contenu, puis boutons, champs, cartes et feuilles.
- **Crème translucide** (sidebar) : la barre latérale, sous un flou.
- **Encre** (text) : texte ; devient le fond en sombre.
- **Taupe sombre** (text-2) : texte secondaire, libellés, placeholder. Le taupe de la planche (#9f887d) assombri pour l'AA.
- **Filets** (border) : séparateurs, contour des boutons et des cartes.
- **Trait fort** (border-strong) : contour des champs, radios, cartes de choix et rail des interrupteurs ; 3:1 sur tous les fonds.
- **Lavis taupe** (fill, fill-strong) : survol et pression, ligne courante de la navigation, segment signalé dans la relecture.
- **Crème chouette** (owl-field ; sombre : kraft) : le disque derrière la chouette, dont le trait sombre exige un fond clair même en thème sombre.
- **États** (danger, warn, ok) : échec, zones à traiter, prêt.

### Named Rules
**The Two Accents Rule.** Le bleu agit et sélectionne, l'orange enregistre. Jamais l'un à la place de l'autre.

**The Blue Text Rule.** Un texte bleu prend accent-text, jamais accent.

**The Warm Neutral Rule.** Aucun gris froid : tout neutre est un mélange d'encre ou de taupe.

## Typography

**Display Font:** Valley Sans Variable (avec system-ui), embarquée via `@fontsource-variable/valley-sans`, sans chargement réseau.
**Body Font:** la même.

**Character:** une seule famille, la hiérarchie vient de la taille et de la graisse (variable de 400 à 700).

### Hierarchy
- **Display** : titre d'écran et titre d'onboarding, une ligne tronquée à l'ellipse.
- **Counter** : le chrono d'enregistrement, chiffres tabulaires.
- **Headline** : titres de section (« Réunions récentes », « Zones d'ombre »), résumé repliable.
- **Body** : listes, réglages ; le texte de transcription passe à 1.6 d'interligne, colonne d'environ 68 caractères.
- **Label** : libellés de champ et nom de locuteur (550 à 650), texte secondaire en 12.5px/400 couleur text-2.

### Named Rules
**The Tabular Time Rule.** Horodatages, durées et compteurs en chiffres tabulaires.

**The One Family Rule.** Valley Sans partout, aucune seconde police.

## Layout

Grille à deux colonnes : barre latérale de 232px, contenu fluide. En-tête d'écran : titre à gauche, action principale à droite, marges de 40px (28px en haut). La relecture ajoute une colonne latérale de 360px (260px sous 1000px de large), où les marges tombent à 24px. Les listes se groupent par jour ; une ligne porte son état à droite, sans bordure ni carte. Espacements en multiples de 4px. L'onboarding occupe toute la fenêtre, sans barre latérale, une colonne centrée, actions en bas à droite.

## Elevation & Depth

Plat par défaut : la profondeur vient du lavis taupe et des filets. Seule la barre latérale est translucide (flou de 30px, saturation 1.8). Les ombres n'existent que pour ce qui flotte au-dessus du contenu.

### Shadow Vocabulary
- **Pop** (`--shadow-pop`) : la notification en bas d'écran.
- **Sheet** (`--shadow-sheet`) : les feuilles modales, sur un voile encre (`--scrim`).
- **Float** (`--shadow-float`) : boutons d'interrupteur et de curseur, carte de zone ouverte, overlay de dictée.

Valeurs exactes et variantes sombres dans `.impeccable/design.json`.

### Named Rules
**The Glass Is Navigation Rule.** Le matériau translucide sert la barre latérale, jamais le contenu.

## Shapes

Trois rayons : contrôle (8px) pour boutons, champs, lignes de navigation et segments ; carte (12px) pour cartes, notification et carte de zone ; feuille (16px) pour les modales. Cercles pour l'étincelle, la chouette, les points d'état et les radios. Les icônes sont dessinées sur 16px, trait de 1.75 à bouts ronds.

## Components

### Buttons
- **Shape:** rayon contrôle, 32px de haut (28px en petit) ; un bouton icône seul garde son dessin mais prend une zone cliquable de 40px.
- **Défaut:** fond blanc, filet, texte encre 14px/550.
- **Primary:** un seul bouton plein bleu par écran ; le reste est discret.
- **Ghost / danger:** fond transparent ; danger en texte rouge, ou plein rouge pour une suppression confirmée.
- **Enregistrement:** plein orange, texte encre.
- **Hover / Focus:** lavis au survol, lavis fort à la pression, 150ms ; focus en anneau bleu de 2px, décalé de 2px (0 sur les champs). Désactivé à 50 % d'opacité.

### Inputs / Fields
- **Style:** fond blanc, trait fort (3:1), rayon contrôle, 32px de haut ; libellé au-dessus en label text-2.
- **Select:** même champ, chevron taupe dessiné à droite.
- **Interrupteur:** tout booléen est une case native avec `role="switch"`, rail 32 × 18px, pastille blanche dans les deux thèmes, bleu quand activé.
- **Radio:** anneau en trait fort qui se remplit de bleu.
- **Contraste élevé:** radios et interrupteurs rendent la main au système (`forced-colors`).

### Navigation
Barre latérale : marque « Barnaby » suivie de l'étincelle, lignes de 32px, icône colorée par fonction (Accueil orange cuit, Relecture bleu étincelle, Réglages taupe), compteur de relecture précédé de l'étincelle. Ligne courante en lavis fort et graisse 650. En bas : pastille d'état (point vert prêt, orange pulsé en enregistrement, bleu en traitement) et bouton « Nouvelle réunion » avec son point orange.

### Cards / Containers
Fond blanc, filet, rayon carte, sans ombre. Les listes, elles, ne sont pas en cartes.

### Audio Player
Bouton lecture icône, temps tabulaire, piste fine de 4px remplie de bleu jusqu'à la tête de lecture, pastille ronde de 14px. Chaque zone à relire pose une étincelle sur la piste, à l'endroit exact où la pastille s'arrêterait. Le segment en cours de lecture s'entoure d'un trait bleu de 2px.

### Sheets
Dialogue natif de 480px au plus, rayon feuille, ombre sheet sur voile ; il monte de 8px en 200ms. Actions alignées à droite.

### Spark (signature)
L'étincelle à quatre branches (SVG, couleur `--spark`) marque tout ce qui attend une action : ligne à relire, segment signalé, compteur, repère sur la piste. Quand la chose est réglée, elle tourne de 90° et s'éteint en 240ms. Elle n'est jamais décorative.

### Owl
Six poses raster (`ui/src/assets/owl/` : idle, listening, thinking, review, success, error) sur un disque crème. Elle respire lentement (5s) ; en succès elle saute une fois. Désactivable dans les réglages. Moments clés seulement : onboarding (160px), écrans vides (72px), fin de traitement, erreurs (32px à côté du message).

### Icons
Jeu maison de dix icônes 16px (accueil, relecture, réglages, enregistrer, micro, étincelle, coche, chevron, lecture, pause), trait 1.75 en `currentColor`, pleines pour enregistrer, étincelle, lecture et pause. La couleur vient de la fonction, pas de l'icône.

## Do's and Don'ts

### Do:
- **Do** garder une seule action pleine par écran, en bleu étincelle.
- **Do** marquer de l'étincelle tout ce qui attend une décision, et seulement cela.
- **Do** faire porter au mouvement un changement d'état : 150ms pour le survol et les bascules, 200ms pour ce qui apparaît, 240ms pour l'étincelle et le saut de la chouette, courbe `--ease`. Seuls le point d'enregistrement et la respiration de la chouette bouclent. Tout s'arrête avec `prefers-reduced-motion`.
- **Do** tenir l'AA sur tout texte et 3:1 sur tout contour de contrôle, dans les deux thèmes.
- **Do** donner 40px de zone cliquable à un petit contrôle isolé.

### Don't:
- **Don't** construire un tableau de bord gris à cartes, ni le look « outil d'IA » sombre à néon.
- **Don't** poser d'eyebrow (petit label au-dessus d'un titre) ni de dégradé décoratif.
- **Don't** mettre la chouette ailleurs qu'aux moments clés ; ailleurs, l'étincelle suffit.
- **Don't** utiliser un caractère (✦, flèche, puce) comme icône : icônes et étincelle sont des SVG.
- **Don't** reprendre le gros contour de la chouette dans l'interface : il reste à la mascotte.
