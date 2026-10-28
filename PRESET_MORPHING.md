# Odin 2 Preset Morphing API

## Overview

The Preset Morphing API allows for real-time interpolation between multiple `OdinPreset` instances. This is useful for:
- Creating dynamic, evolving sounds.
- Implementing "X/Y Pad" interfaces.
- Procedural audio generation based on game state.

## Core Concepts

### 2D Scatter Morphing

The system moves away from simple linear interpolation (A to B) and instead uses a **geometric** approach. You define any number of "source" presets, each with an (X, Y) coordinate. You can then request a blended preset at any target (X, Y) location.

The blending algorithm uses **Inverse Distance Weighting (IDW)**. Presets closer to the target point have more influence.

## API Reference

### `OdinPreset::morph_2d`

Interpolates between multiple presets based on 2D coordinates.

```rust
pub fn morph_2d(sources: &[(OdinPreset, f32, f32)], x: f32, y: f32) -> Self
```

- **sources**: A slice of tuples, where each tuple contains:
  - The source `OdinPreset`.
  - The X coordinate (f32).
  - The Y coordinate (f32).
- **x**: The target X coordinate.
- **y**: The target Y coordinate.
- **Returns**: A new `OdinPreset` with interpolated parameters.

### `OdinPreset::interpolate`

Linearly interpolates between two presets.

```rust
pub fn interpolate(&self, other: &Self, t: f32) -> Self
```

- **other**: The target preset to morph towards.
- **t**: Interpolation factor (0.0 = self, 1.0 = other).

## Examples

### Basic 4-Corner Morphing

This setup mimics a standard X/Y controller where each corner corresponds to a different sound.

```rust
// 1. Load your presets
let p_top_left = OdinPreset::load("A.odin")?;
let p_top_right = OdinPreset::load("B.odin")?;
let p_btm_left = OdinPreset::load("C.odin")?;
let p_btm_right = OdinPreset::load("D.odin")?;

// 2. Define geometry
let sources = vec![
    (p_top_left, -1.0, 1.0),
    (p_top_right, 1.0, 1.0),
    (p_btm_left, -1.0, -1.0),
    (p_btm_right, 1.0, -1.0),
];

// 3. Morph at input coordinates
// Mouse input, gamepad stick, etc.
let input_x = 0.5; 
let input_y = -0.2;

let blended = OdinPreset::morph_2d(&sources, input_x, input_y);
```

## Lecture des presets upstream et audit

Le chargement `.odin` est disponible avec la feature `std`. Le décodeur lit
récursivement le flux binaire JUCE : comptes entiers compressés, noms UTF-8,
types `var` void/undefined, int32/int64, double, booléen, chaîne, tableau et
bloc binaire. Un flux tronqué ou une longueur `var` incohérente produit une
erreur, pas un preset partiel. Les booléens JUCE utilisent bien les marqueurs
2=true et 3=false. Le marqueur 9 désigne undefined, pas un double.

`OdinPreset::from_value_tree_audited(&tree)` renvoie `(preset, audit)`.
`audit.unread` contient les paramètres audio, propriétés et descendants
inconnus réellement non lus. Conserver un nœud dans `ValueTree::root` ne le
fait jamais passer pour consommé : les chemins consommés sont enregistrés
lors des affectations typées. Les champs d'interface connus sont exposés
dans `preset.ui`, séparément des paramètres audio. L'audit est un contrôle
de **lecture du patch**, pas une preuve que tous les DSP produisent le même
signal que le plugin C++.

Commande récursive, sans chemin usine codé en dur :

```sh
cargo run -p odin2-core --features std --example audit_presets -- \
  "/chemin/vers/Factory Presets" --list-ignored
```

Le CLI imprime chaque `NON_LU` avec fichier et chemin de propriété, les
migrations appliquées, les exclusions explicites upstream et un bilan.
Il échoue si un fichier est invalide, si un champ n'est pas lu, ou si le
chemin ne contient aucun `.odin`. Le CLI ne lance pas le moteur audio.

### Contrat de migration et valeurs

Sources de la traduction : `Source/PluginProcessorReadPatch.cpp:20-171`,
`PluginProcessorMigratePatch.cpp:20-77`,
`PluginProcessorNonAudioParams.cpp:20-241`, `AudioValueTree.h:18-425`,
`GlobalIncludes.h:89-115,172-175` et
`PluginProcessorValueChange.cpp:18-50,1212-1257` du dépôt Odin 2 upstream.

- Les anciennes entrées audio `oscN_carrier_ratio`,
  `oscN_modulator_ratio` et `pitchbend_amount` sont migrées vers les champs
  non audio et **priment** sur ceux-ci, même dans un patch de version récente,
  comme dans `migratePatch`.
- Pour un schéma de migration antérieur à 5, le chargement utilise la base
  init upstream avant les valeurs présentes. Les seize écarts d'arrondi de
  l'asset init par rapport aux valeurs déclarées sont conservés.
  Un chargement autonome commence toujours avec des valeurs déterministes;
  il ne dépend pas d'un patch chargé auparavant dans le plugin.
- `amp_width` est obsolète depuis le schéma 2 et n'a pas de DSP actuel.
  `modwheel` et `pitchbend` ne sont volontairement pas restaurés par
  `readPatch::checkLoadParameter`. L'audit les indique dans
  `ignored_by_upstream` et les exclut de `loaded_parameters`; ce ne sont
  pas des champs inconnus masqués par une whitelist générée.
- Les tableaux absents reprennent les générateurs upstream : wavedraw,
  193 valeurs de sinus à amplitude 0.9; chipdraw, 32 valeurs à ±0.875;
  specdraw, 49 partiels dont seul le premier vaut 1. Ils sont exposés
  directement dans chaque `OscillatorParams` et interpolés échantillon
  par échantillon.
- `fx_positions` contient les positions de delay/phaser/flanger/chorus/reverb,
  dans cet ordre, avec défaut `[0,1,3,2,4]`. L'interpolation change l'ordre
  complet au seuil 0.5, sans fabriquer de positions intermédiaires.
- Les synchronisations conservent les indices sérialisés. Le numérateur
  musical vaut `index + 1`; les dénominateurs sont
  `[1,2,4,8,12,16,24,32,48]`. `sync_ratio` renvoie une durée en rondes;
  secondes = `240 / BPM * ratio`, Hz = `BPM / (240 * ratio)`.
- Les volumes et gains sont en dB, les fréquences en Hz, les temps ADSR
  en secondes, `rev_delay` en millisecondes. Le glide reste le coefficient
  upstream, pas une durée arbitraire.

Les IDs d'oscillateurs, filtres, LFO, distorsion et arpégiateur sont ceux
sauvegardés par upstream, pas des indices Rust contigus. En particulier
oscillateur 1=aucun et 2=analogique; filtre 1=bypass, 2=LP24,
24=Korg LP, 25=Korg HP; LFO 2=triangle et 7=sample-and-hold;
distorsions 1..5=Clamp/Fold/Zero/Sine/Cube; directions arp 10..90.

### Catalogue pour l'hôte

Les `*Params` et enums sont `Copy` et utilisables en `no_std`.
`AUDIO_PARAMETERS` expose les 258 paramètres déclarés par upstream avec
nom, type, minimum, maximum et défaut en unités physiques.
La macro publique `odin_audio_parameters!(callback, ...)` fournit la même
liste avec chemins structurés; les défauts du preset et ses accesseurs
`set_audio_param` / `audio_param` consomment cette liste, sans deuxième
table de noms. Les paramètres non audio (types, routage de l'éditeur,
dessins, synchronisation et matrice) restent des champs typés distincts.
Les opérations de chargement et de morphing allouent et sont réservées
au contrôle hors callback audio; le moteur reçoit les structures runtime.
