# API hôte d'odin2-core

## Chargement et configuration

`PresetConfig`, qui perdait les types d'oscillateurs, le routage, les modulations et les effets, est supprimé. Utiliser `EngineConfig`, disponible avec ou sans `std`. Les champs conservent les unités physiques du patch : décibels, hertz, secondes, valeurs bipolaires ou normalisées selon le paramètre.

Avec `std`, `OdinPreset::load(path)` décode le ValueTree JUCE et applique ses migrations. `OdinEngine::load_preset(&preset)` renvoie un `Result` : un identifiant de source/destination inconnu est une erreur, pas une modulation ignorée. `EngineConfig::from_preset` permet d'inspecter ou de modifier la configuration avant chargement.

Sans `std`, construire un `EngineConfig` directement, puis appeler `load_config`. Le moteur utilise `alloc` ; `no_std` ne signifie pas absence d'allocateur à la construction.

`new`, `load_config`, `load_preset` et `set_sample_rate` sont **hors callback audio**. Le chargement prépare les tables dessinées partagées et arrête les voix/queues du patch précédent. Les sources C++ externes et le système de fichiers ne sont pas utilisés par le rendu Rust.

`OdinEngine` est `Send` : une instance préparée peut être transférée au thread audio en Rust sûr.
Les tables dessinées immuables utilisent `Arc`, cloné seulement au chargement, jamais par
échantillon. Le test `prepared_engine_moves_to_audio_thread_without_changing_render` compare
le rendu transféré au rendu local et vérifie zéro allocation dans le rendu du thread destinataire.
Le remplacement et la destruction d'une instance restent hors callback.

## Pilotage générique en temps réel

Le catalogue est partagé avec le parseur, et non reconstruit dans une seconde liste propre à l'hôte. Il inclut oscillateurs, filtres/drive, enveloppes, LFO, gains, pad XY, effets et paramètres audio de l'arpégiateur.

```rust
use odin2_core::{OdinEngine, ParameterId, SynthEngine};

let mut engine = OdinEngine::new(48_000.0);
// Résoudre une fois hors callback et conserver ces identifiants.
let cutoff = ParameterId::from_name("fil1_freq").unwrap();
let drive = ParameterId::from_name("fil1_saturation").unwrap();
let delay_mix = ParameterId::from_name("delay_wet").unwrap();

engine.note_on(48, 100);
engine.set_parameter(cutoff, 800.0).unwrap();
engine.set_parameter(drive, 0.25).unwrap();
engine.set_parameter(delay_mix, 0.3).unwrap();
let mut output = [0.0; 256];
engine.process(&mut output, 2);
```

- `ParameterId::all()` parcourt le catalogue sans allocation.
- `descriptor()` expose nom, minimum, maximum, défaut et nature continue/entière/booléenne.
- `engine.parameter(id)` lit la valeur physique courante.
- `engine.set_parameter(id, value)` affecte les voix actives **et** futures, sans recharger le patch ni relancer les enveloppes. Les valeurs finies sont bornées au domaine JUCE ; entiers arrondis et booléens seuillés à 0,5. NaN/infini sont rejetés.
- `engine.set_parameters(&[(id, value)])` applique un lot en validant tout avant de recalculer les voix : un seul recalcul DSP pour un ensemble de paramètres émotionnels, et aucune mutation si une valeur est invalide.
- Le nom est stable pour la sérialisation. L'index interne de `ParameterId` ne l'est pas entre versions.
- Aucune conversion linéaire 0..1 n'est présentée comme la normalisation JUCE : certains contrôles upstream ont une courbe asymétrique. Un hôte émotionnel choisit explicitement sa courbe et transmet la valeur physique.

Les sélections non audio (type d'oscillateur, forme dessinée, ordre d'effets, nombre d'unissons) se configurent dans `EngineConfig` au chargement. `set_modulation_row` modifie une ligne de matrice de façon transactionnelle, sans allocation ; une ligne ou un identifiant invalide laisse la matrice active inchangée.

## MIDI et tempo

`note_on`/`note_off`, `control_change`, `pitch_bend`, `channel_pressure` et `poly_aftertouch` sont synchrones et sans allocation. Le pitch bend est signé, centre zéro, dans [-8192,8191]. Le sustain libère toutes les voix associées à une touche, y compris les répétitions et unissons. Les modes poly, mono avec redéclenchement et legato utilisent le mode du patch.

`set_tempo(bpm)` pilote les modules synchronisés sans remettre leur phase à zéro. `all_notes_off` lance les releases ; `all_sound_off` coupe et réinitialise les états DSP.

`process` écrit de l'audio entrelacé ; les canaux supplémentaires à la stéréo restent à zéro. En mono, le canal gauche du chemin stéréo est utilisé. Passer zéro canal laisse le buffer à zéro. Aucun événement n'est retardé au bloc suivant : segmenter le buffer aux positions MIDI souhaitées.

## Migration d'Harmonium

Aucun fichier du dépôt Harmonium n'est modifié ici. Son adaptateur doit remplacer la construction du `PresetConfig` réduit par le chargement du patch complet, traiter les erreurs de matrice et conserver des `ParameterId` pour le pilotage émotionnel. Les chargements de presets doivent rester hors du callback ; seuls les paramètres numériques, le tempo et les événements MIDI y sont mis à jour.
