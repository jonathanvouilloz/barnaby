# DESIGN — Barnaby

Direction : spec section 10.1 (« Apple-like »). Une app outil, calme, qui s'efface derrière le travail de l'utilisateur.

## Principes
- Une action principale par écran, en bouton plein. Le reste en boutons discrets.
- Densité confortable : l'écran de relecture affiche beaucoup de texte, qui doit rester lisible avant tout.
- Les matériaux translucides servent à la barre latérale et aux feuilles modales, pas au contenu.
- Les mouvements accompagnent un changement d'état (150 à 220 ms, en ease-out). Rien ne boucle, sauf l'indicateur d'enregistrement. Tout est coupé avec `prefers-reduced-motion`.
- Pas d'eyebrow ni de dégradés décoratifs.

## Typographie
Police système uniquement : `system-ui, -apple-system, "Segoe UI Variable Text", "Segoe UI", Cantarell, "Noto Sans", sans-serif`. Chiffres tabulaires pour les horodatages (`font-variant-numeric: tabular-nums`).

| Rôle | Taille / graisse |
|---|---|
| Titre d'écran | 22px / 600 |
| Titre de section | 15px / 600 |
| Corps | 14px / 400, interligne 1.5 |
| Secondaire | 12.5px / 400, couleur `--text-2` |

## Tokens (définis dans `ui/src/app.css`)
| Token | Clair | Sombre |
|---|---|---|
| `--bg` | `#f5f5f7` | `#1c1c1e` |
| `--surface` | `#ffffff` | `#2c2c2e` |
| `--sidebar` | `rgb(236 236 240 / .8)` | `rgb(44 44 46 / .72)` |
| `--text` | `#1d1d1f` | `#f5f5f7` |
| `--text-2` | `#6e6e73` | `#a1a1a6` |
| `--border` | `rgb(0 0 0 / .1)` | `rgb(255 255 255 / .12)` |
| `--accent` | `#0a66d8` | `#3d9bff` |
| `--danger` | `#c62828` | `#ff6b60` |
| `--warn` (zones d'ombre) | `#9a6400` | `#ffcc4d` |
| `--ok` | `#1f7a3a` | `#4cd774` |

Rayons : 6px (contrôles), 10px (cartes), 14px (feuilles). Espacements : multiples de 4px.

## Accessibilité
Contraste AA sur tous les textes, focus visible (anneau de 2px `--accent`), navigation complète au clavier, `aria-live` pour les changements d'état (enregistrement, traitement fini).

## Mascotte
Chouette SVG provisoire (`ui/src/lib/Owl.svelte`), en attendant celle de l'auteur. Un état par phase du système (spec section 10.4), désactivable dans les réglages.
