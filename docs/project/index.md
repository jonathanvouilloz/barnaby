# Index : Barnaby

Reprise : [handoff](handoff.md) · Produit : [PRD](../SPEC.md) · [Architecture](architecture.md) · [Décisions](decisions.md) · Journal des changements : `CHANGELOG.md`

## Fonctionnalités

| Fonctionnalité | Statut | Fiche | Point d'entrée code |
|---|---|---|---|
| Dictée | livré | [dictation](../features/dictation.md) | `src-tauri/src/dictation.rs` |
| Réunion : capture et traitement | livré | [meeting-capture](../features/meeting-capture.md) | `crates/core/src/pipeline.rs` |
| Relecture des zones d'ombre | livré | [review](../features/review.md) | `crates/core/src/review.rs` |
| Envoi : dossier et webhook | livré | [destinations](../features/destinations.md) | `crates/core/src/send.rs` |
| Transcription en ligne (Mistral) | livré | [cloud-transcription](../features/cloud-transcription.md) | `crates/core/src/mistral.rs` |
| Premier lancement | livré | [onboarding](../features/onboarding.md) | `ui/src/onboarding/Onboarding.svelte` |
