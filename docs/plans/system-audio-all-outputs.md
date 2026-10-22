# Capturer le son de toutes les sorties audio

## Contexte

Test réel sous Windows : une réunion Google Meet n'est pas enregistrée dans `system.wav`, alors qu'une vidéo YouTube l'est. Cause : `open_loopback` (`crates/engine/src/capture.rs:188`) n'écoute que la sortie par défaut de Windows. Or Meet joue souvent sur une autre sortie : la sortie mains libres d'un casque Bluetooth, la sortie de communication, ou un haut-parleur choisi dans Meet. Linux a le même défaut : `parec --device=@DEFAULT_MONITOR@` n'écoute que la sortie par défaut.

Objectif (décision de Jon) : enregistrer **toutes les sorties actives**, mélangées dans `system.wav`, quelle que soit la configuration. Une sortie qui apparaît pendant la réunion est prise en compte. En bonus de confort : une option pour exclure une sortie (par exemple celle qui joue de la musique).

## Approche

Un seul fil d'exécution « système » remplace la capture actuelle. Il :
1. liste les sorties actives au démarrage, puis toutes les 3 secondes ;
2. ouvre l'écoute de chaque nouvelle sortie non exclue, et ferme celle d'une sortie disparue ;
3. toutes les 100 ms, convertit le son reçu de chaque sortie en 16 kHz mono (`downmix` et `Resampler` de `crates/engine/src/dsp.rs`), le mélange, puis envoie le résultat au rédacteur existant (`spawn_writer` / `write_track`) en 16 kHz mono.

Le rédacteur, les niveaux, l'alerte « système silencieux » et le remplissage de silence ne changent pas.

Le mélange est réglé sur l'horloge. À chaque tour, le fil produit les échantillons attendus depuis le début, moins 200 ms de marge pour absorber les retards de livraison. Pour chaque échantillon, il additionne la file de chaque sortie (0 si elle est vide), puis borne le résultat à [-1, 1]. Une file qui dépasse 1 s de retard perd ses plus vieux échantillons, pour que deux horloges audio qui dérivent ne fassent pas grossir la file sans fin.

### Plateformes
- **Windows (et macOS via cpal)** : `cpal::default_host().output_devices()` ne renvoie que les sorties actives (`DEVICE_STATE_ACTIVE`, vérifié dans cpal 0.18.2). L'identifiant vient de `Device::id()` (format `wasapi:{…}`, stable) et le nom de `Device::description()?.name()`. L'écoute passe par `input::open` (`crates/engine/src/input.rs`), comme aujourd'hui.
- **Linux (PulseAudio ou PipeWire)** : `pactl list sinks` donne `Name:` et `Description:` pour chaque sortie. On lance un `parec --device=<name>.monitor --format=s16le --rate=16000 --channels=1 --raw` par sortie, comme le `start_parec` actuel. `pactl` vient du même paquet que `parec` (`pulseaudio-utils`, déjà requis). Le profil mains libres d'un casque Bluetooth change le nom de la sortie : le nouveau scan toutes les 3 s le rattrape.

## Fichiers

1. **`crates/engine/src/outputs.rs`** (nouveau) : le code propre à chaque plateforme.
   - `pub struct Output { pub id: String, pub name: String }` ;
   - `pub fn list() -> Vec<Output>` ;
   - `fn open(id, tx) -> Result<Source>`. `Source` vaut `cpal::Stream` hors Linux et `std::process::Child` sous Linux (un `parec` à tuer à la fermeture), avec `rate` et `channels`.
   - Le fil de lecture de `parec`, aujourd'hui dans `start_parec`, est déplacé ici sans changement de logique.
   - Exporté depuis `crates/engine/src/lib.rs`.
2. **`crates/engine/src/capture.rs`** :
   - `Recorder::start(dir, excluded: &[String])` lance le fil système, qui possède les sources et les ferme à l'arrêt ;
   - `struct Mixer` (pur, testable) : `push(id, mono)`, `take(n) -> Vec<f32>`, borne de 1 s par file ;
   - suppression de `open_loopback`, de `start_parec`, du champ `parec` de `Recorder` et des `#[cfg]` qui vont avec ;
   - `NO_SYSTEM` reste émis si aucune sortie n'a pu être ouverte après le premier scan.
3. **`crates/core/src/model.rs`** :
   - `MeetingSettings.excluded_outputs: Vec<String>` (défaut vide ; `#[serde(default)]` est déjà présent, donc les réglages existants restent lisibles) ;
   - nouveau type `OutputDevice { id, name }`, recopié dans `ui/src/lib/api.ts`.
4. **`src-tauri/src/meeting.rs`** : passe `app.settings().meeting.excluded_outputs` à `Recorder::start`.
5. **`src-tauri/src/commands.rs`** et `lib.rs` : commande `list_outputs` qui renvoie `Vec<OutputDevice>`, ajoutée au `generate_handler!`.
6. **UI (bonus)** :
   - `ui/src/screens/Settings.svelte`, section Réunion : « Sorties écoutées », une case par sortie, cochée par défaut ; décocher ajoute l'identifiant à `excluded_outputs`. Une sortie exclue mais absente reste dans la liste, pour ne pas perdre le choix quand le casque est débranché.
   - `ui/src/lib/api.ts` (`listOutputs`), `ui/src/lib/mock.ts` (deux sorties factices) et textes FR et EN dans `ui/src/lib/i18n.ts`.
7. **Docs** :
   - le plan est copié dans `docs/plans/` ;
   - une ligne dans `docs/project/decisions.md` : toutes les sorties plutôt que la capture par processus, non vérifiée sur plusieurs sorties ;
   - mise à jour de la section capture de `docs/SPEC.md` si elle dit « sortie par défaut ».

## Ordre (test d'abord)

1. Tests de `Mixer` qui échouent, puis `Mixer`. Cas à couvrir :
   - deux sources additionnées ;
   - une source vide donne des zéros ;
   - une source arrivée en retard s'ajoute à la position courante ;
   - une file de plus de 1 s est tronquée ;
   - la somme est bornée à [-1, 1].
2. `outputs.rs` : `list`, `open` et l'analyse de la sortie de `pactl list sinks`, avec un test sur un texte réel de `pactl`.
3. Fil système dans `capture.rs`, branchement dans `meeting.rs`. Mise à jour du test `header_is_flushed_while_recording` si sa construction de `Shared` change.
4. Réglage, commande `list_outputs` et UI.
5. Docs.

## Vérification

- `cargo test -p barnaby-core`, `cargo test -p barnaby-engine` (tests `Mixer` et analyse `pactl`), `cargo clippy --workspace --all-targets -- -D warnings`, `cd ui && bun run check && bun test`.
- Sur ce serveur Linux, si `pactl` répond : lister les sorties et vérifier que `list()` les renvoie. Sinon, le signaler comme non testé.
- Windows : `cargo check --target x86_64-pc-windows-msvc -p barnaby-engine` si la cible est installée, sinon la CI. Le vrai test revient à Jon, sur son PC :
  - réunion Meet avec casque Bluetooth, puis avec haut-parleurs ;
  - YouTube en même temps sur une autre sortie ;
  - vérifier que `system.wav` contient les deux, puis qu'une sortie exclue n'y est plus.

## Hors périmètre

- Capture par processus Windows : plan B si un cas réel échoue encore.
- Annulation d'écho entre le micro et les haut-parleurs.
- macOS : cpal y expose aussi les sorties, mais rien n'est testé (Phase 1 = Windows et Linux).
