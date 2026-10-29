# Envoi : dossier et webhook

> Statut : livré · Plan : `docs/plans/dictation-polish.md`

## À quoi ça sert
Livrer le compte rendu d'une réunion validée là où l'utilisateur le veut : un dossier local ou un webhook signé.

## Cas d'usage
1. Je choisis un dossier (sélecteur) et j'exporte une réunion déjà validée.
2. Je configure un webhook, je génère le secret, je teste l'envoi.

## Règles
Un envoi raté ne perd rien : état `failed`, cause et nouvelle tentative (invariant 6 d'architecture). Les secrets sont dans le trousseau de l'OS. Un avertissement précède une validation sans destination. Détail : `docs/SPEC.md` section 15.

## Écrans et points d'entrée
`crates/core/src/destination.rs`, `crates/core/src/send.rs`, `crates/core/src/export.rs`, `ui/src/screens/Settings.svelte`.

## Tests
`crates/core/src/destination.rs`, `crates/core/src/send.rs`, `crates/core/src/export.rs`.

## Limites connues
Webhook : 30 s d'attente sur un hôte muet ; régénérer le secret l'écrase sans confirmation. `hasDestination` accepte un chemin d'espaces que le Rust refuse.
