# Site de présentation — cadrage

Validé par Jon le 2026-10-22.

## Pourquoi
Barnaby devient public. Une seule page dit ce que c'est, montre la vidéo de Jon et mène au téléchargement. Esprit de [handy.computer](https://handy.computer/) : un geek a construit un outil gratuit, le voici, servez-vous. Pas une page de vente : pas d'urgence, pas de prix, pas de témoignages inventés.

## Pour qui
Indépendants et petites équipes francophones qui tombent sur le lien (réseaux, GitHub, bouche-à-oreille) sans connaître Jon.

## Contenu, dans l'ordre
1. En-tête : chouette, « Barnaby », liens GitHub et Télécharger.
2. Accroche : la chouette animée en grand, ce que fait l'app en une phrase simple, bouton « Télécharger pour <système> » qui mène à la section L'installer, sans rien télécharger (demande de Jon du 2026-10-23 : un téléchargement au premier clic fait peur).
3. Vidéo de Jon (YouTube intégré en mode sans cookies). Tant qu'elle n'existe pas, la section est cachée.
4. À quoi ça sert : dictée au curseur, et réunions (enregistrer, transcrire, relire, résumé).
5. Comment ça marche : une réunion en trois étapes (écoute, texte, vérification), chacune avec une capture de l'app sur des données d'exemple.
6. Comment l'utiliser : installer (trois cartes Windows, Mac, Linux, celle du système du visiteur en premier et « Recommandé pour toi » ; le fichier part au clic sur sa carte), choisir ses modèles à l'onboarding, raccourci de dictée, lancer une réunion.
7. Questions fréquentes : gratuit ? où vont mes données ? quel ordinateur ? macOS ?
8. Qui l'a fait : photo de Jon, une phrase, liens jonlabs.ch et X (@jonvolio).
9. Remerciements : Handy, Whisper, Parakeet, sherpa-onnx, Mistral.
10. Don : cachée tant que Jon n'a pas de lien.
11. Pied de page : GitHub, licence MIT, lien vers Jon Labs.

## Contraintes
- Français seulement (fr-CH), tutoiement, ton de l'app (`PRODUCT.md`).
- Look de l'app : couleurs, police et chouette de `DESIGN.md`. Clair et sombre.
- Liens de téléchargement (`releases/latest/download/Barnaby-windows-setup.exe`, `Barnaby-macos-arm64.dmg`, `Barnaby-linux-amd64.deb`) : un tag `v*` publie la release avec ces noms (job `release` de la CI). Les tailles et le numéro de version écrits dans les cartes sont à mettre à jour à chaque release.
- Texte compréhensible par un enfant de 7 ans : ce que ça fait, pas comment c'est fait (demande de Jon).
- Aucun pistage, aucun cookie hors vidéo YouTube sans cookies.
- Page légère, lisible sur téléphone.

## Hors périmètre
Anglais, blog, documentation détaillée, compteur de téléchargements, newsletter, section don active.

## Succès
Un inconnu comprend en 30 secondes ce que fait Barnaby, sait s'il peut l'installer, et télécharge le bon fichier sans aide.
