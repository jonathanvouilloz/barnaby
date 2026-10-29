# Transcription en ligne (Mistral)

> Statut : livré · Plan : `docs/plans/cloud-transcription.md`

## À quoi ça sert
Transcrire les réunions par l'API Mistral Voxtral quand le poste est trop lent ou la qualité locale insuffisante.

## Cas d'usage
1. Je colle ma clé Mistral, je la teste, je choisis le modèle en ligne pour mes réunions.

## Règles
Réunions seulement ; la dictée reste locale. Le mode `local` n'ouvre jamais le réseau (invariant 3 d'architecture). Cadrage complet : `docs/project/cloud-transcription.md`.

## Écrans et points d'entrée
`crates/core/src/mistral.rs`, `ui/src/lib/MistralSheet.svelte`.

## Tests
`crates/core/src/mistral.rs`.

## Limites connues
`check_key` jamais essayée contre la vraie API ; la fenêtre d'erreur n'a jamais été capturée.
