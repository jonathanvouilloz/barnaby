---
version: 1
slug: "ui-src-app-svelte"
primary_target: "ui/src/App.svelte"
related_targets: ["ui/src/screens","ui/src/Overlay.svelte"]
---

## Scope
Toute la fenêtre principale de Barnaby (accueil, nouvelle réunion, enregistrement, file et écran de relecture, réglages) et le nouvel onboarding. Mode : Operate. L'overlay de dictée et la fenêtre de notes suivent le même monde.

## Audience and task
Jon puis le public (PRODUCT.md). Tâches : démarrer une réunion, relire les passages douteux, régler. L'onboarding : micro et modèles, test de dictée en direct, tour des fonctions.

## Direction contract
THESIS: Barnaby se lit comme Things, la chouette en plus : une app de bureau aérée où une seule chose attire l'œil, ce qui attend ta décision. Refuse le tableau de bord gris à cartes et le look « outil d'IA » sombre à néon.
OWN-WORLD: Contenu blanc #fdfdfd, barre latérale translucide aux tons crème (#f8ebdc) et kraft (#decfc5), texte encre #0f0c2f, secondaire taupe (#9f887d assombri pour l'AA), bleu étincelle #0170ec pour les actions et la sélection, orange bec #fcaa4f pour l'enregistrement. Valley Sans partout, grands titres gras, petites icônes colorées par fonction dans la barre latérale, lignes de liste sans bordure. Sombre : fond encre #0f0c2f.
STORY: À l'ouverture on voit tout de suite ce qui attend (réunions à relire, enregistrement en cours), on agit d'un clic, puis l'app s'efface.
FIRST VIEWPORT: Barre latérale à gauche (Accueil, Relecture avec son compteur ✦, Réglages, bouton « Nouvelle réunion » en bas). Zone principale : titre d'écran 28px gras, puis la liste des réunions par jour, chaque ligne avec son état ; l'action principale en haut à droite.
FORM: Pick (candidat le mieux classé, non tiré), Things comme modèle de qualité. Signature : l'étincelle ✦ marque tout ce qui attend une action, et s'anime brièvement (rotation et éclat) quand c'est réglé. seed: none (unavailable)
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Mascot
Aux moments clés seulement : onboarding, écrans vides, fin de traitement, erreurs. Ailleurs, seulement l'étincelle.

## Open
Logo final et chouette détourée : à fournir par Jon. La boucle vidéo a un fond blanc et un filigrane Kling.
