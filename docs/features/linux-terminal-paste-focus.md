# Linux : collage dans les terminaux et pastille sans vol de focus

> **Statut** : DONE (testé à la main, Linux seulement) · **Livré** : 2026-11-16 · **Commits** : `b7fcce3` (branche `fix/linux-terminal-paste-focus`, base `333b486`)

## Ce que ça fait
Sous Linux, le texte dicté se colle aussi bien dans un terminal (Ptyxis, VTE, xterm) que dans une appli graphique (VS Code). Sur GNOME Wayland, la pastille d'enregistrement ne vole plus le focus de la fenêtre où l'on dicte.

## Comment ça marche
1. `inject::paste_text` écrit le texte dans CLIPBOARD (`set_private`), puis dans PRIMARY (`arboard` `LinuxClipboardKind::Primary`, best effort, simple `log::info` si échec).
2. `inject::simulate_paste` (Linux) envoie **Shift+Insert** et non Ctrl+V : les terminaux prennent Ctrl+V littéralement, Wayland ne dit pas quelle app a le focus, et les applis graphiques collent CLIPBOARD sur Shift+Insert.
   - Wayland : `wtype -M shift -k Insert -m shift`, puis `dotool` (`key shift+insert` sur stdin), puis `xdotool key --clearmodifiers shift+Insert`.
   - X11 : `xdotool` seul. Dernier recours : `enigo_paste` (Shift pressé, Insert cliqué, Shift relâché).
   - Le portail RemoteDesktop (`portal::paste`) envoie les keycodes evdev Shift 42 / Insert 110.
3. `main.rs` : si `GDK_BACKEND` est absent, `XDG_CURRENT_DESKTOP` contient `GNOME` et `WAYLAND_DISPLAY` existe, l'app se lance sous XWayland (`GDK_BACKEND=x11`, posé avant tout thread). Mutter donne le focus à toute fenêtre affichée ; sous XWayland la pastille reste infocusable, au premier plan et positionnée.

## Carte du code
> Mise à jour : 2026-11-16

| Fichier | Rôle |
|---|---|
| `src-tauri/src/inject.rs` | `paste_text` (CLIPBOARD + PRIMARY), `simulate_paste`, `run_tool`, `enigo_paste` : Shift+Insert sous Linux |
| `src-tauri/src/portal.rs` | `paste()` : Shift+Insert via le portail RemoteDesktop (keycodes 42/110) |
| `src-tauri/src/main.rs` | force `GDK_BACKEND=x11` sur GNOME Wayland |

### Décisions clés
- Shift+Insert plutôt que Ctrl+V, PRIMARY remplie en plus — compatible terminaux sans détecter l'app (détail → `docs/project/decisions.md`, 2026-11-16)
- XWayland forcé sur GNOME Wayland, `GDK_BACKEND` utilisateur prioritaire — pastille sans focus (remplace la décision du 2026-11-15 « pastille au centre »)

## Comment débuguer
- **Par où entrer** : `inject::simulate_paste` (quel outil réussit) ; `main.rs` pour le backend GDK.
- **Ce qu'on observe** : log `primary selection: …` si PRIMARY échoue ; `echo $GDK_BACKEND` dans l'environnement du process ; `wl-paste -p` / `xclip -o -selection primary` pour PRIMARY.
- **Pannes connues** : rien collé dans un terminal → l'ancien Ctrl+V n'est plus envoyé, vérifier que l'outil (wtype, dotool, xdotool, portail) a réussi ; pastille qui vole le focus → `GDK_BACKEND` défini à `wayland` par l'utilisateur, ou bureau non GNOME.
- **Pièges** : une appli qui lie Shift+Insert à autre chose ne collera pas ; le portail demande l'autorisation une fois (jeton `remote-desktop.token`) ; build local, voir Configuration.

## Configuration
- Variables lues : `GDK_BACKEND`, `XDG_CURRENT_DESKTOP`, `WAYLAND_DISPLAY`. Aucune migration.
- Gotchas de build Linux local : `cargo` hors du PATH (`~/.cargo/bin`) ; `target/{debug,release}/deps` doit exister avant le build de `sherpa-rs-sys` ; le binaire release demande `--features tauri/custom-protocol`.

## Tests et vérifications
| Vérif | Commande | Résultat réel |
|---|---|---|
| GNOME Wayland + Ptyxis + VS Code | manuel (Jon) | colle dans les deux, pastille sans vol de focus |
| Windows, macOS | — | non testé (chemin inchangé) |
| Suite Rust / clippy | — | non relancé |

## Limites et dettes
- La sélection PRIMARY est écrasée à chaque dictée.
- Flou possible sous XWayland en échelle fractionnaire (125 % avait écarté cette voie le 2026-11-15) ; non revérifié.
- Aucun test automatisé du collage.
- `docs/project/handoff.md` dit encore « pastille au centre » (décision du 2026-11-15) : obsolète, à corriger par `/wrap-session`.
