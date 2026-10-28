# Fidélité Odin 2 — protocole et état des preuves

## Résultats mesurés

La campagne a été exécutée par l'intégrateur sur macOS (Apple clang 21, arm64) avec JUCE,
tuning-library et nlohmann/json aux révisions épinglées ci-dessous. Le vrai processeur
Odin 2 C++ compile sans fenêtre (`odin_reference`) et les sept presets sont rendus par le
vrai `processBlock`. Résumé (scénario `note`, 44,1 kHz, 4 s, deltas Rust − C++) :

| Preset | ΔRMS | Δcentroïde | Δflatness | Enveloppe L2 |
|---|---|---|---|---|
| Bass Crusher | 0,00 dB | 0,2 % | <1e-4 | 0,00 |
| DeepBass | −0,00 dB | 0,0 % | ~1e-9 | 0,00 |
| Sausage Bassener | +1,0 dB | 18 % | 5e-4 | 0,52 |
| Analog Bass | −0,6 dB | 11 % | 1e-5 | 0,49 |
| Darken Nature | +0,4 dB | 4,6 % | <1e-8 | 0,31 |
| Drifter | +0,0 dB | 2,1 % | 1e-5 | 0,08 |
| Clouded | +0,0 dB | 6,3 % | <1e-8 | 0,08 |

Plafond d'aléa mesuré : deux lancements indépendants du C++ seul divergent de
L2 enveloppe 0,27–0,36 sur Analog Bass et Sausage Bassener (phase aléatoire à
`voiceStart`), et de 0,00 sur DeepBass. Preuve que l'écart résiduel des presets
vient des tirages aléatoires et non du DSP : avec `osc{1,2,3}_reset=1` (phases
déterministes, surcharge `--set` identique des deux côtés via `scripts/bisect.py`),
Clouded passe de 9 % à 0,05 % d'écart de centroïde et Analog Bass de 11 % à 0,5 %,
enveloppes L2 divisées par 2 à 4. Le C++ tire phases et bruits de `rand()` après
`srand(0)` ; Rust emploie un xorshift à graine fixe : les deux sont déterministes,
mais leurs tirages diffèrent, ce qui domine la mesure sur les presets à battements
lents, cascade d'effets modulés ou bruit. Les étages ont été isolés avec
`scripts/bisect.py` ; les modules sont comparés individuellement par
`scripts/compare_{filters,osc,fx}.py` avec les vraies unités `Source/audio`
(résultats dans `PORTING.md`).

Le reste de ce document décrit le protocole, les seuils fixés avant mesure et les limites.

## Exécution centralisée

Prérequis : Python 3.9+, compilateur C++17, CMake 3.22+, Cargo et dépendances Rust disponibles. La référence complète exige les sous-modules et ressources attendus par le CMake amont. L'outil ne télécharge rien, n'initialise aucun sous-module et ne modifie pas l'amont.

Depuis la racine de ce dépôt :

```sh
python3 scripts/fidelity.py \
  --upstream /chemin/vers/odin2-upstream \
  --out target/fidelity-run-001
```

`--upstream` est obligatoire et configurable; aucun chemin personnel n'est codé en dur. `--out` doit être vide pour exclure toute réutilisation involontaire d'anciennes captures. `CXX` peut désigner un autre compilateur pour les modules. `--presets /chemin/banque` remplace la banque usine amont. Fréquence et durée sont configurables avec `--rate 44100 --seconds 4`; une durée inférieure à trois secondes est refusée.

Pour mesurer en présence de WAV de référence obtenus séparément dans Odin C++ :

```sh
python3 scripts/fidelity.py \
  --upstream /chemin/vers/odin2-upstream \
  --skip-reference-build \
  --reference-dir /chemin/captures-cpp \
  --out target/fidelity-import-001
```

L'origine C++ des fichiers importés doit être établie par leur producteur; l'outil ne la devine pas. Il enregistre leurs empreintes SHA-256 et ne fabrique jamais une référence à partir du rendu Rust. Les références absentes restent absentes. Les rendus `biquad_*` et `adsr_gate` sont toujours tentés, même avec `--skip-reference-build`; les autres modules exigent `--juce`.

Le code retour vaut zéro uniquement si les quatorze couples preset/scénario disposent d'une référence exploitable et respectent les seuils de triage, si toutes les comparaisons de modules listées dans `modules-comparison.csv` (biquad, ADSR, ladder, un pôle, distorsion, oscillateur analogique) existent et respectent leurs seuils, et si aucune erreur n'a été consignée. Sans `--juce`, les comparaisons ladder/un pôle/distorsion/oscillateur sont absentes, donc le code retour est non nul. Un échec CMake suivi de mesures modules réussies reste un résultat incomplet. Ce statut n'est pas une certification acoustique.

### Commandes sous-jacentes

Le script consigne les arguments exacts de chaque processus dans `report.json` et dans les journaux. Les principales commandes, à exécuter par l'intégrateur, sont :

```sh
cmake -S tools/fidelity -B target/reference-build \
  -DODIN_UPSTREAM=/chemin/vers/odin2-upstream -DCMAKE_BUILD_TYPE=Release
cmake --build target/reference-build --target odin_reference --config Release --parallel 2
cargo build -p odin2-core --release --features std --example fidelity_render
```

L'exemple peut également être appelé directement :

```sh
cargo run -p odin2-core --release --features std --example fidelity_render -- \
  preset '/chemin/Bass Crusher [RM].odin' /chemin/Bass_Crusher_note.wav 44100 note 4
```

Il utilise l'API publique conservée : `OdinEngine::new(f32)`, `load_preset(&OdinPreset)` et `SynthEngine::{note_on,note_off,process}`, puis `set_tempo(120.0)` pour expliciter le tempo hôte. L'erreur de chargement du preset est propagée. Ni périphérique audio ni accès au moteur privé ne sont requis.

## Référence complète C++ sans fenêtre

`tools/fidelity/CMakeLists.txt` ajoute le vrai projet amont, puis lie la bibliothèque de code partagé `Odin2` à `odin_reference`. Toutes les sources `Source/audio` utilisées par ce projet restent celles de l'amont; elles ne sont pas copiées dans ce dépôt. Les symboles et dépendances JUCE d'interface restent nécessaires à la liaison du processeur existant, mais le banc ne crée jamais d'éditeur, de fenêtre, d'hôte audio ou de périphérique audio. Il initialise les services JUCE nécessaires au processeur.

`tools/fidelity/reference.cpp` lit le vrai ValueTree `.odin`, le convertit en état XML JUCE et appelle `setStateInformation`. Ce chemin exécute les migrations et reconstruit les tables draw (`PluginProcessorMisc.cpp:104-171`); appeler seulement `readPatch` ne reconstruirait pas ces tables. Les notes passent par `MidiBuffer` et le vrai `processBlock` (`PluginProcessorProcess.cpp:20-179`). Le transport fourni est en lecture, à 120 BPM, 4/4; aucun paramètre du preset n'est neutralisé pour faciliter une comparaison.

Les logs `reference-configure.log` et `reference-build.log` fournissent la raison exacte d'un blocage CMake, JUCE, SDK ou édition de liens. Un diagnostic séparé, `reference-source-probe.log`, tente aussi `CXX -std=c++17 -fsyntax-only Source/audio/ADSR.cpp` sur la vraie unité amont, sans en-têtes générés ni remplacement : il capture notamment le premier include JUCE manquant si CMake s'arrête d'abord sur un autre sous-module. Ce diagnostic de source non configurée ne remplace pas la tentative de construction complète et son échec n'en préjuge pas le résultat. Aucun faux moteur C++ ni remplacement numérique de JUCE n'est utilisé en cas d'échec.

## Comparaisons de modules réelles sans JUCE

La solution de repli compile **les vrais fichiers amont** :

- `Source/audio/Filters/BiquadFilter.cpp:24-44` et son en-tête;
- `Source/audio/ADSR.cpp:27-144,192-225,228-254` et son en-tête.

Le script crée dans le répertoire de résultats un arbre de liens symboliques vers ces quatre fichiers. Pour ce sous-ensemble uniquement, les en-têtes d'agrégation JUCE/globaux sont remplacés dans cet arbre par `cassert`, `cmath` et `jassert -> assert`. Ces deux modules n'utilisent aucune approximation numérique JUCE : aucun calcul, coefficient, branche DSP ou fonction mathématique n'est remplacé. Les sources originales ne sont ni copiées, ni éditées, ni réécrites. Les assertions restent actives; `-fno-fast-math` est demandé. Ce repli ne prétend pas mesurer les filtres non linéaires dépendants de JUCE ni le synthétiseur complet.

`tools/fidelity/modules.cpp` et le mode `modules` de l'exemple Rust appliquent :

| Module réel | Stimulus et paramètres | Comparaison |
|---|---|---|
| Biquad | Impulsion unité, sinus 440 Hz à 0,25, balayage logarithmique 20 Hz–min(20 kHz, 0,45 Fs), chacun une seconde | Échantillons bruts et RMSE |
| ADSR | Gate ouvert à zéro et fermé à 0,75 s; A=10 ms, D=100 ms, S=0,4, R=200 ms; capture deux secondes, modulation nulle | Échantillons bruts et RMSE |

Le biquad reçoit `a1=-0,5`, `a2=0,125`, `b0=0,125`, `b1=0,25`, `b2=0,125`. Le but est de comparer la récurrence réelle, et non deux recettes indépendantes de coefficients. Les stimuli sont quantifiés une fois en `f32`, puis lus identiquement par C++ et Rust. Les trames et entrées doivent coïncider; une divergence empêche la comparaison. C++ conserve ses calculs internes `double`, Rust conserve ceux de son module : aucune réduction artificielle de précision de la référence.

Tolérances maximales absolues, fixées **avant** toute mesure :

- Biquad : `2e-6`. Les coefficients et le signal sont bornés et le filtre est stable; ce seuil laisse une marge aux arrondis `f32` et à la différence entre formes directes II/non transposée et transposée. Il ne légitime pas une erreur de gain/récurrence.
- ADSR : `2e-3`. La récurrence exponentielle et l'attaque accumulent les arrondis `f32`; le passage à zéro au seuil `0,001` peut survenir à une trame différente. Le seuil n'efface pas le CSV ni la RMSE : une différence systématique de courbe doit être examinée même sous ce seuil.

Ces quatre cas minimaux (biquad ×3, ADSR) ne couvrent ni modulation d'enveloppe, ni bouclage, ni retrigger/legato. Ils ne sont pas une preuve de fidélité globale. Le mode JUCE externe ci-dessous étend la couverture avec les vraies unités de filtres, distorsion et oscillateur analogique.

## Modules audio réels avec JUCE externe (`--juce`)

Révisions épinglées par les gitlinks de l'amont (`git ls-tree HEAD libs/...`) :

| Dépendance | Dépôt | Révision |
|---|---|---|
| JUCE | `https://github.com/juce-framework/JUCE` | `2f980209cc4091a4490bb1bafc5d530f16834e58` |
| tuning-library | `https://github.com/surge-synthesizer/tuning-library.git` | `8eb7232ac24ac2f97a052063161c0c4f5fd763e3` |
| nlohmann/json | `https://github.com/nlohmann/json.git` | `960b763ecd144f156d05ec61f577b04107290137` |

Les sous-modules `libs/clap-juce-extensions` et `libs/JUCELV2` ne sont pas utilisés par ce banc : la cible plugin y est recréée sans CLAP ni LV2, avec exactement la liste des unités `Source/*.cpp` du `CMakeLists.txt` amont (lue à la configuration, pas recopiée). Aucun sous-module JUCE n'est exigé par ce chemin; si JUCE demande un composant absent, l'échec figure dans `logs/reference-configure.log`.

```sh
python3 scripts/fidelity.py \
  --upstream /chemin/vers/odin2-upstream \
  --juce /tmp/odin-fidelity-juce-2f980209 \
  --tuning /tmp/odin-fidelity-tuning-8eb7232a \
  --json /tmp/odin-fidelity-json-960b763e \
  --out target/fidelity-juce-001
```

`--audio-modules-only` limite la compilation C++ aux modules audio (sans processeur ni GUI). Les trois dépendances externes sont lues telles quelles; l'amont n'est pas modifié. Comme `Source/gui/JsonGuiProvider.h` inclut `../../libs/json/...` par chemin relatif et que ce sous-module amont est vide, le script crée `upstream-view/` : dossiers réels, fichiers symlinkés vers l'amont, `libs/json` et `libs/tuning-library` pointant vers les clones externes. Aucun fichier C++ n'est copié ni modifié; une vue déjà présente fait échouer l'exécution.

La cible `odin_audio_modules` (`tools/fidelity/audio_modules.cpp`) compile toutes les unités `Source/audio/**/*.cpp` avec le vrai `JuceHeader.h` généré et le vrai `juce_dsp` (approximations `FastMathApproximations` incluses). Elle pilote :

| Famille | Unités amont | Cas et paramètres |
|---|---|---|
| Ladder | `LadderFilter` `FILTERTYPE` 0–5 (LP24, LP12, BP24, BP12, HP24, HP12) | 1 kHz, `setResControl(0,25)`, sans modulation ni overdrive, stimuli impulsion/sinus/balayage |
| Un pôle VA | `VAOnePoleFilter` LP/HP | 1 kHz, mêmes stimuli |
| Distorsion | `OversamplingDistortion` Clamp/Fold/Zero/Sine/Cube | seuil 0,5, dry/wet 0,75, sans modulation, mêmes stimuli |
| Oscillateur analogique | `AnalogOscillator` Saw/Pulse/Triangle/Sine après `loadWavetablesFromConstData` | fréquence entrée par échantillon : 220 Hz constant, balayage exponentiel 55–1760 Hz; glide, drift et modulation nuls, largeur 0,5 |

Côté Rust : `LadderFilter` + `Filter::{set_cutoff,set_resonance}`, `VAOnePoleFilter::update`, `OversamplingDistortion::{set_algorithm,set_threshold,set_dry_wet}` et `PresetOscillator::{configure,set_frequency,process}` avec `OscillatorType::Analog`. Les sorties sont écrites en CSV `frame,input,output` avec les mêmes noms dans `modules-cpp/` et `modules-rust/`.

Tolérances maximales absolues fixées avant toute mesure : Ladder/un pôle `1e-3`, distorsion `2e-4`, oscillateur `2e-3`. Elles couvrent seulement des arrondis `f32`, l'écart attendu entre `tan`/`exp` approchés et la précision `double` du C++ pour des signaux d'amplitude ≤ 0,25 à 1 (impulsion). Un dépassement est un écart mesuré à analyser, pas une raison d'élargir le seuil a posteriori. Les modulations, les filtres SEM/Diode/Korg/Comb/Formant, les oscillateurs Wavetable/Multi/Vector/Chiptune/FM/PM/Draw, le LFO et les effets de chaîne sont couverts par les bancs par domaine `scripts/compare_{filters,osc,fx}.py`, qui compilent les mêmes unités amont ; seuls les sources à phase libre (bruit, drift, arp Random) restent non comparables échantillon par échantillon, et seule la référence processeur complète compare les sept presets au moteur entier.

Limite de déterminisme : `reference.cpp` réinitialise `srand(0)` après la construction du processeur (le constructeur `Voice` rappelle `srand(time)`) et après le chargement d'état. Les affectations aléatoires faites pendant la construction avant cette ligne et `std::random_device` de l'arpégiateur, si un preset l'active, restent hors de ce contrôle.

## Sept presets et stimuli MIDI

La recherche accepte le nom exact ou son suffixe auteur entre crochets et exige un seul fichier correspondant :

| Nom du contrat | Fichier usine inspecté |
|---|---|
| Bass Crusher | `Bass/Bass Crusher [RM].odin` |
| DeepBass | `Bass/DeepBass [Photonic].odin` |
| Sausage Bassener | `Bass/Sausage Bassener.odin` |
| Analog Bass | `Bass/Analog Bass [tx].odin` |
| Darken Nature | `Pads/Darken Nature [RM].odin` |
| Drifter | `Pads/Drifter.odin` |
| Clouded | `Pads/Clouded [RM].odin` |

Chaque preset est chargé dans un nouveau moteur pour chaque scénario, en stéréo flottant 32 bits, sans normalisation et sans écrêtage ajouté. Les deux moteurs reçoivent les mêmes événements, vélocité 100 :

- `note` : note MIDI 48 à 0,25 s; note-off à 2,25 s.
- `pattern` : note 48 à 0,25 s, note 55 à 0,75 s, note-off 48 puis note 60 à 1,25 s, note-off 55 à 1,75 s, note-off 60 à 2,25 s. Les recouvrements exercent le comportement polyphonique/legato du preset.

Les instants sont arrondis à la trame la plus proche; les blocs ne dépassent pas 256 trames. Le rendu Rust découpe les blocs aux événements; C++ place les événements à leur offset exact dans le `MidiBuffer`. La durée par défaut de quatre secondes inclut 250 ms avant déclenchement et 1,75 s de queue après le dernier note-off; elle ne prétend pas épuiser toutes les réverbérations. Pour une comparaison longue des pads, relancer avec une durée supérieure et des références de même durée.

Les noms WAV attendus sont `Bass_Crusher_note.wav`, `Bass_Crusher_pattern.wav`, etc., en remplaçant les espaces du nom du contrat par `_`. Une fréquence, un nombre de trames ou un format non pris en charge sont refusés; aucun alignement ou rattrapage de gain automatique ne masque un décalage.

## Mesures et tolérances de triage

Les mesures ne somment jamais les canaux avant de calculer leur énergie, pour ne pas annuler un signal stéréo en opposition de phase.

- **RMS** : racine de l'énergie moyenne des deux canaux sur la capture entière, silence initial et queue inclus. Écart comparé en dB.
- **Centroïde spectral** : moyenne des puissances FFT des deux canaux, fenêtres Hann de 2048 trames non recouvrantes, DC exclue; moyenne des fréquences pondérée par cette puissance. La dernière fenêtre incomplète est exclue du spectre, pas du RMS.
- **Flatness** : moyenne géométrique / moyenne arithmétique des mêmes puissances, plancher numérique `1e-30`, DC exclue. Ce n'est pas une moyenne de flatness fenêtre par fenêtre.
- **Enveloppe** : RMS stéréo en fenêtres successives de 10 ms, dernière fenêtre partielle incluse; erreur L2 relative à l'enveloppe de référence.
- **Diagnostics d'échantillons** : RMSE et écart absolu maximal non alignés; ils restent visibles même pour une source avec phase aléatoire. Aucun résultat n'est recalé pour les améliorer.

| Mesure | Seuil de triage | Justification et limite |
|---|---|---|
| Écart RMS | ≤ 0,5 dB absolu | Détecter les erreurs sensibles de gains et d'empilement de voix sans exiger l'identité des phases |
| Centroïde | ≤ 5 % relatif, dénominateur ≥ 20 Hz | Détecter une modification importante du filtre ou du contenu harmonique |
| Flatness | ≤ 0,03 absolu | Détecter une substitution tonale/bruit; le ratio relatif serait instable près de zéro |
| Enveloppe | ≤ 10 % L2 relative | Détecter des écarts d'attaque, sustain et release sans être hypersensible à une trame |

Ce sont des seuils d'ingénierie initiaux pour prioriser les écarts, **ni des seuils psychoacoustiques validés, ni une tolérance d'identité numérique, ni une preuve que deux sons sont identiques**. Les comparaisons modules sont plus strictes lorsque phase et conditions initiales sont contrôlées. Une référence dont le RMS est inférieur à `1e-8` n'est pas acceptée comme preuve : le rapport exige un diagnostic. Un signal non fini ou vide est une erreur. Le pic brut est conservé, y compris au-dessus de 1; aucun normaliseur ne masque un gain incorrect.

L'amont utilise des états de phase/bruit/drift qui peuvent ne pas être reproductibles entre deux processus malgré `srand(0)`; ce banc ne remplace pas les générateurs amont et ne prétend pas contrôler le hasard système JUCE. Un faible écart perceptuel avec une forte erreur échantillon doit être expliqué, pas présenté comme une équivalence numérique. Si la fidélité d'un preset aléatoire est contestée, plusieurs captures C++ indépendantes doivent établir la variance de référence avant de revoir les seuils.

## Artifacts et lecture du rapport

Le répertoire de résultats contient :

- `logs/*.log` : commandes et sorties exactes, y compris configuration/compilation échouée;
- `stimuli/*.csv` : entrées réellement données aux filtres;
- `modules-cpp/*.csv`, `modules-rust/*.csv` : sorties brutes des vrais modules;
- `modules-comparison.csv` : erreurs numériques et tolérances de modules;
- `rust/*.wav`, `cpp/*.wav` quand disponibles : captures sans normalisation;
- `captures/{rust,cpp}/*_envelope.csv`, `*_spectrum.csv` : enveloppes et spectres mesurés;
- `metrics.csv` : mesures de chaque capture disponible;
- `comparison.csv` : deltas de presets, statut de référence, diagnostics numériques et seuils;
- `report.json` : commandes, codes retour, empreintes des modules/sources de référence inspectées et des presets, mesures et erreurs;
- `rapport.md` : synthèse française générée, sans statut de conformité inventé.

Une absence de référence ne produit ni zéro d'erreur, ni succès implicite. La seule livraison des outils ne remplit pas une campagne de mesures : l'intégrateur doit exécuter la commande, conserver les artifacts et citer les résultats réellement obtenus.
