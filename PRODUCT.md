# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

Application de bureau Tauri 2 (webview) pour Windows, Linux et macOS. L'interface suit les conventions d'une app de bureau (barre latérale, feuilles, raccourcis clavier), pas celles d'un site.

## Users
D'abord Jon, qui s'en sert chaque jour pour dicter et pour enregistrer ses réunions avec ses clients. Ensuite le public : indépendants et petites équipes qui téléchargent l'app open source sans connaître son auteur, sur des machines de toutes puissances.

## Product Purpose
Deux modes sur un même moteur : la dictée (un raccourci, on parle, le texte arrive là où est le curseur) et la réunion (enregistrer n'importe quel appel, séparer les voix, relire les passages douteux, envoyer le résultat). Succès : un texte juste sans retouche, et une réunion relue en quelques minutes.

## Positioning
Local d'abord : par défaut rien ne quitte la machine, et le passage en ligne est un choix explicite, annoncé (fournisseur, données, prix). Indépendant de la plateforme d'appel : capture du son de l'ordinateur, aucun bot dans la réunion. L'humain tranche : l'app montre ce dont elle n'est pas sûre.

## Operating Context
L'app vit surtout cachée : raccourcis globaux, petite bulle de dictée, fenêtre de notes par-dessus l'appel, notification en fin de traitement. La fenêtre principale sert à démarrer une réunion, relire, régler. Spec complète : `docs/SPEC.md`.

## Capabilities and Constraints
Voir `docs/SPEC.md` (sections 6 à 15) et `docs/project/architecture.md`. Interface en français par défaut, anglais disponible. Pas de compte, pas de mobile.

## Brand Commitments
- Nom : Barnaby. Mascotte : une chouette ronde et douce, créée par Jon (planche et boucle vidéo dans `design/references/`), sous licence CC BY 4.0.
- Ton : tutoiement, complice et simple ; phrases complètes, pas de jargon. Anglais : direct et chaleureux.
- Références visuelles imposées par Jon : `design/references/refs.md`.

## Evidence on Hand
Planche de la mascotte (`design/references/owl-sheet.png`), boucle animée (`design/references/owl-loop.mp4`, avec filigrane Kling à retirer). Aucun témoignage, aucun chiffre d'usage : ne rien inventer.

## Product Principles
1. Disparaître pendant le travail, être limpide quand on ouvre la fenêtre.
2. Dire ce qui sort de la machine avant que ça sorte.
3. Montrer l'incertitude au lieu de la cacher.
4. Marcher sur une machine modeste.

## Accessibility & Inclusion
WCAG AA, navigation complète au clavier, lecteurs d'écran, `prefers-reduced-motion` respecté.
