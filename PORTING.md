# Portage d'Odin 2 dans odin-rs

Suivi des sections d'Odin 2 (amont : le dépôt C++ `odin2-upstream` du projet, GPLv3, version 2.3.4, migration de patch 5) vers leur état dans `odin2-core`, avec les preuves réellement exécutées. Les implémentations Rust sont des traductions de code C++ ; aucune source amont n'est copiée dans ce dépôt.

## Contrats d'exécution

- Construction et chargement de preset hors callback audio (`OdinEngine::new`, `load_config`, `load_preset`, `set_sample_rate`). Aucune allocation, verrou ni E/S dans `process`, `note_on`/`note_off`, `control_change`, `pitch_bend`, `set_parameter` : prouvé par un allocateur de test global (`tests/engine_runtime.rs`, 0 opération d'allocation sur MIDI + rendu + mises à jour de paramètres).
- Déterminisme : deux moteurs identiques rendent le même buffer, blocs fragmentés ou non (`tests/engine_runtime.rs`). Le hasard amont (`srand(time)`, `random_device`) est remplacé par des graines fixes documentées ci-dessous ; c'est le seul écart volontaire.
- `cargo test -p odin2-core --features std` : 107 tests verts. `cargo test -p odin2-core` (sans `std`) : 90 verts. `cargo clippy --all-targets --features std` et `--lib` : zéro avertissement. Le crate reste `#![no_std]` avec `alloc`.

## Phase 1 — chemin de voix complet

| Section Odin 2 | État dans odin-rs | Preuves |
|---|---|---|
| `audio/Oscillators/*` (11 types + drift + VA one-pole + DC blocker) | `dsp/oscillators/preset_oscillator.rs`, `analog.rs`, `wavetable.rs`, `wavetable2d.rs`, `multi.rs`, `vector.rs`, `chiptune.rs`, `fm.rs`, `pm.rs`, `noise.rs`, `drift.rs`, `lfo.rs` ; tables dessin partagées `draw.rs` (`Arc`, construites au chargement, instance `Send`) | `python3 scripts/compare_osc.py` : 579 scénarios C++ réel (JUCE) vs Rust, erreur max ≤ 7,5e-5 ; familles analog/wt2d/multi à ~1e-7 ; 48 et 96 kHz idem |
| `audio/Filters/*` (ladder 6 types, SEM, diode, Korg LP/HP, comb, formant, ring mod, base `OdinFilterBase.h`) | `dsp/filters/preset_filter.rs`, `ladder.rs`, `sem.rs`, `diode.rs`, `korg35.rs`, `comb.rs`, `formant.rs`, `biquad.rs`, `va_one_pole.rs` | `python3 scripts/compare_filters.py` : 195 cas × 3 stimuli, erreur max 1,1e-4 (20–30× sous le seuil 1e-3) ; Diode 1,7e-6, Comb 5,8e-8 ; 48/96 kHz idem |
| `audio/ADSR.cpp`, `audio/Amplifier.cpp` | `dsp/envelopes/adsr.rs` (modulation ADSR par la matrice, legato/retrig/poly), ampli dans `voice.rs` (gain dB, pan, vélocité, unisson) | banc `fidelity.py` : `adsr_gate` erreur max 6,0e-4 (seuil 2e-3) ; `compare_osc.py` scénarios `voice` (glide/unison/bend) ≤ 2,6e-4 |
| `audio/Voice.h` (routage osc→fil1/fil2, fil1→fil2, `to_amp`, distorsion de voix avant env amp, sources de modulation) | `voice.rs` | scénario `voice` du banc oscillateurs ; preset complets ci-dessous |
| `VoiceManager`, unisson (`PluginProcessor.h:313-331`) | `voice_manager.rs` (priorités sustain/libres/release/plus ancienne, distributions pan/detune/gain exactes) | `tests/engine_runtime.rs` (répétitions de notes + sustain + unisson ×4) |
| `OdinArpeggiator` | `arpeggiator.rs` (séquence bornée sans allocation ; Random avec graine fixe) | lecture ligne à ligne de `gui/OdinArpeggiator.cpp` ; non comparé au C++ (random_device non reproductible), Random documenté |
| Glide, lissage des contrôles (`Process.cpp:66-105`), fréquences de filtre ×64 semi-notes | `voice.rs`, `preset_filter.rs`, `engine.rs` | mesure : le bug de glide (facteur `pow(glide, 44800/sr)` manquant) a été trouvé par `compare_osc.py` puis corrigé |

Écarts amont reproduits volontairement : drift/PWM/sous-sync du pulse (`doSquare` ×3), boucle de lecture PM hors table, plafond noise 0,49·fs — tous relevés par le banc après correction initiale.

## Phase 2 — matrice de modulation

| Section | État | Preuves |
|---|---|---|
| `gui/ModMatrix.cpp` (équations 23-110), identifiants 120-1029, `PluginProcessorSetModPointers.cpp` | `mod_matrix.rs` : 9 lignes, source+scale+2 destinations, `montant × |montant|`, IDs amont exacts, rejet transactionnel des IDs inconnus (`InvalidModulationId`) | `cargo test mod_matrix::tests` ; lecture des sources |
| Temporalité : sources DSP de l'échantillon précédent, destinations globales sur la voix la plus récente, XY clampé avant la matrice | `engine.rs::process`, `voice.rs::process_with_modulation` | bissection `scripts/bisect.py` : sans matrice, niveau Rust = C++ (voir phase 5) |
| Montants sérialisés | `ModMatrixRow::{dest1_amount,dest2_amount,scale_amount}` : le fichier écrit `amount_0_row_N` (dest1), `amount_1_row_N` (dest2), `amount_2_row_N` (scale) — décalage de nom amont (`ProcessorInitializerList.h:345-366`) | bug trouvé par le banc preset (−21 dB sur Bass Crusher), corrigé, re-mesuré à 0,00 dB |

## Phase 3 — chaîne d'effets

| Section | État | Preuves |
|---|---|---|
| `audio/FX/Delay`, Chorus, Flanger, Phaser, ZitaReverb | `dsp/effects/{delay,chorus,flanger,phaser,reverb}.rs` | `python3 scripts/compare_fx.py` : 219 scénarios, erreur max 2,4e-7 (chorus/flanger/phaser bit-exacts) ; 22,05→192 kHz |
| Ordre configurable des 5 slots (`Process.cpp:441-472`, priorité `else if` amont sur les doublons) | `dsp/effects/chain.rs` | scénarios `chain_*` du banc (ordres par défaut/inversés/mélangés, doublons) |
| Distorsion de voix (`OversamplingDistortion` après ampli) | `dsp/effects/distortion.rs` + `voice.rs` | banc modules : 5 algorithmes ≤ 6,1e-9 (impulsion) |
| Quirks amont reproduits : Sine retombe sur Cube, asymétrie de rate phaser L/R, flanger `(in+delayed)*0.5`, phase initiale chorus droit 0.25 / flanger droit 0.5, `set_rtlow` salit `c_dirty` | `dsp/effects/*` | scénarios dédiés du banc, tous sous seuil |

Bugs corrigés grâce au banc : rayon phaser 1.32→1.25 (`PluginProcessorMisc.cpp:228`), séquence set_time/reset/sync du delay au chargement, pré-delay signé de la réverb.

## Phase 4 — parseur de presets

| Section | État | Preuves |
|---|---|---|
| ValueTree JUCE binaire (comptes compressés, `var`, récursivité) | `preset/value_tree.rs` | `cargo run --example audit_presets` |
| `ReadPatch` + `MigratePatch` (versions < 5, base init, anciens ratios, `pitchbend_amount`) | `preset/parser.rs` | migrations 1 et 4 détectées sur le corpus |
| Catalogue 258 paramètres audio (`AudioValueTree.h`) | `preset/types.rs` : macro unique `odin_audio_parameters!`, défauts amont, enums aux identifiants exacts (osc 2/3/4/5/6/7/8/15/20/21/22 ; filtres 1/2/…/40 ; LFO 0–22 ; distorsion 1–5 ; arp 10–90) | audit des 260 presets d'usine |
| Audit « zéro non lu » | `examples/audit_presets.rs` | exécution réelle : `PRESETS=260 PARAMETRES_CHARGES=64299 NON_LUS=0 IGNORES_AMONT=527 ERREURS=0` ; les seuls ignorés sont ceux que l'amont ignore (`amp_width` obsolète, `modwheel`/`pitchbend` refusés par `checkLoadParameter`) |
| Dessins (wavedraw 193, chipdraw 32, specdraw 49) et état XML | reconstruits au chargement via `DrawTableBank` | compare_osc (familles draw ≤ 2,4e-5) |
| `modwheel`/`pitchbend` ne restaurés pas par le patch (comportement `readPatch` amont) | ignorés au chargement, pilotables par l'hôte | bug de −21 dB trouvé par le banc et corrigé |

## Phase 5 — API hôte

| Section | État | Preuves |
|---|---|---|
| `PresetConfig` (moteur réduit à 3 oscillateurs analogiques) | supprimé ; remplacé par `config::EngineConfig` (`Copy`, no_std) et `parameters::ParameterId` sur le catalogue unique 258 | `tests/engine_runtime.rs` ; `API_HOTE.md` |
| Pilotage émotionnel en temps réel | `OdinEngine::{set_parameter,set_parameters,parameter,config}` : affecte voix actives et futures sans recharger ni relancer les enveloppes | test `live_volume_parameter_changes_an_already_sounding_note` |
| Matrice à chaud | `set_modulation_row` transactionnel sans allocation | `cargo test mod_matrix` |
| Temps réel | voir contrats ci-dessus | allocateur de test : 0 allocation |

## Fidélité des presets (banc `scripts/fidelity.py`)

Référence : le vrai processeur Odin 2 C++ compilé sans fenêtre avec JUCE à la révision du gitlink amont, presets chargés par le vrai `setStateInformation` (migrations + tables draw incluses). Mesures scenario `note` (RMS dBFS relatif, centroïde relatif, enveloppe L2 relative) :

| Preset | ΔRMS | Δcentroïde | Δflatness | Enveloppe L2 |
|---|---|---|---|---|
| Bass Crusher | 0,00 dB | 0,2 % | <1e-4 | 0,00 |
| DeepBass | −0,00 dB | 0,0 % | ~1e-9 | 0,00 |
| Sausage Bassener | +1,0 dB | 18 % | 5e-4 | 0,52 |
| Analog Bass | −0,6 dB | 11 % | 1e-5 | 0,49 |
| Darken Nature | +0,4 dB | 4,6 % | <1e-8 | 0,31 |
| Drifter | +0,0 dB | 2,1 % | 1e-5 | 0,08 |
| Clouded | +0,0 dB | 6,3 % | <1e-8 | 0,08 |

Plafond d'aléa amont mesuré : deux lancements indépendants du **C++ seul** divergent entre eux de L2 enveloppe 0,27–0,36 sur Analog Bass et Sausage Bassener (phases aléatoires à `voiceStart`), tandis que DeepBass est reproductible (0,00). Preuve que l'écart résiduel vient des tirages aléatoires et non du DSP : avec `osc{1,2,3}_reset=1` (phases déterministes au note-on, surcharge `--set` appliquée aux deux moteurs), Clouded passe de 9 % à **0,05 %** d'écart de centroïde (220,5 vs 220,6 Hz) et Analog Bass de 11 % à **0,5 %** (96,0 vs 95,5 Hz), enveloppes L2 divisées par 2 à 4 (Darken Nature : 0,04). Le C++ tire ses phases/bruits de `rand()` après `srand(0)` ; Rust utilise un xorshift à graine fixe (exigence de déterminisme du dépôt). Chacun est déterministe de son côté, mais leurs tirages diffèrent : sur les presets à battements lents, cascade d'effets modulés ou bruit (Sausage Bassener), ce décalage d'initialisation domine la mesure. Le niveau RMS des sept presets reste ≤ 1 dB dans tous les cas. Protocole complet, seuils et limites : `FIDELITE.md`. Outils : `scripts/fidelity.py` (presets + modules), `scripts/compare_{filters,osc,fx}.py` (modules par domaine), `scripts/bisect.py` (isolation d'étage et de paramètre par `--set`).

## Écarts connus et non couverts

- `SurgeReverb` amont n'est pas porté (l'amont ne l'utilise pas dans le chemin audio).
- Arpégiateur global Random : graine fixe au lieu de `random_device` (exigence de déterminisme du dépôt).
- Le pitch bend amont est appliqué via `pitch_semitones` ; les tuning tables alternatifs (`Tunings::Tuning`) ne sont pas portés (le banc utilise l'accordage standard, comme les presets d'usine).
- La réverb garde la constante amont `6.2832f` ≠ 2π (fidélité au C++, `#[allow]` documenté).
- FMA : le C++ compilé avec contraction FP active peut diverger de Rust sur le delay (≈1e-3 sur des temps entiers en échantillons) ; le banc est construit avec `-ffp-contract=off`.
