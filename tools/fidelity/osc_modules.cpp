// GPL-3.0-or-later. Banc de comparaison oscillateurs / LFO: les VRAIES unités
// Source/audio/Oscillators/** d'Odin 2 (WavetableOsc2D, Multi, Vector, Chiptune,
// FM, PM, WavetableOsc1D draw, LFO, AnalogOscillator) avec leur vrai JUCE.
//
// Usage: odin_osc_modules SPEC STIMULI OUT RATE
//   SPEC: une ligne par scénario "nom type stimulus clé=valeur ..." (voir scripts/compare_osc.py)
//   STIMULI: dossier de CSV (frame,input) = fréquence de base en Hz par échantillon
//   OUT: un CSV (frame,input,output) par scénario + tables.txt, mappings.txt, morph.txt, draw_*.f32
#include "audio/Oscillators/AnalogOscillator.h"
#include "audio/Oscillators/ChiptuneOscillator.h"
#include "audio/Oscillators/FMOscillator.h"
#include "audio/Oscillators/LFO.h"
#include "audio/Oscillators/MultiOscillator.h"
#include "audio/Oscillators/NoiseOscillator.h"
#include "audio/Oscillators/PMOscillator.h"
#include "audio/Oscillators/VectorOscillator.h"
#include "audio/Oscillators/WavetableContainer.h"
#include "audio/Oscillators/WavetableOsc1D.h"
#include "audio/Oscillators/WavetableOsc2D.h"
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <functional>
#include <iomanip>
#include <iostream>
#include <map>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

struct Spec {
    std::string name, type, stim;
    std::map<std::string, double> kv;
    double get(const std::string& key, double fallback) const {
        auto it = kv.find(key);
        return it == kv.end() ? fallback : it->second;
    }
    int geti(const std::string& key, int fallback) const { return static_cast<int>(get(key, fallback)); }
};

// Générateur déterministe partagé avec le Rust (mêmes opérations entières, flottants exacts).
void draw_values(std::uint32_t seed, int count, float amplitude, float* output) {
    std::uint32_t x = seed * 2654435761u + 12345u;
    for (int i = 0; i < count; ++i) {
        x = x * 1664525u + 1013904223u;
        const float unit = static_cast<float>((x >> 8) & 0xffff) / 32768.0f - 1.0f;
        output[i] = unit * amplitude;
    }
}

// Les 3 tables de chaque famille, générées comme à PluginProcessorConstructor.cpp:188-190.
void make_draw_tables(WavetableContainer& tables, std::uint32_t seed) {
    for (int osc = 0; osc < 3; ++osc) {
        float wave[WAVEDRAW_STEPS_X], chip[CHIPDRAW_STEPS_X], spec[SPECDRAW_STEPS_X];
        draw_values(seed * 16 + osc * 3 + 0, WAVEDRAW_STEPS_X, 0.9f, wave);
        draw_values(seed * 16 + osc * 3 + 1, CHIPDRAW_STEPS_X, 0.9f, chip);
        draw_values(seed * 16 + osc * 3 + 2, SPECDRAW_STEPS_X, 1.0f, spec);
        for (float& value : spec) value = (value + 1.0f) * 0.5f; // [0,1) comme l'éditeur spectral
        tables.createWavedrawTable(osc, wave, 44100);
        tables.createChipdrawTable(osc, chip, 44100);
        tables.createSpecdrawTable(osc, spec, 44100);
    }
}

std::vector<Spec> read_specs(const std::filesystem::path& path) {
    std::ifstream input(path);
    if (!input) throw std::runtime_error("SPEC illisible: " + path.string());
    std::vector<Spec> specs;
    std::string line;
    while (std::getline(input, line)) {
        if (line.empty() || line[0] == '#') continue;
        std::istringstream words(line);
        Spec spec;
        words >> spec.name >> spec.type >> spec.stim;
        std::string token;
        while (words >> token) {
            const auto eq = token.find('=');
            if (eq == std::string::npos) throw std::runtime_error("clé=valeur attendu: " + token);
            spec.kv[token.substr(0, eq)] = std::stod(token.substr(eq + 1));
        }
        specs.push_back(std::move(spec));
    }
    return specs;
}

// Pointeurs de modulation: tout à zéro sauf ce que le scénario demande.
struct Mods {
    float exp = 0, lin = 0, bend = 0, glide = 0, pwm = 0, pos = 0, detune = 0, spread = 0;
    float x = 0, y = 0, arp = 0, fm = 0, carrier = 0, modulator = 0, unison = 1;
    float lfo_exp = 0;
};

void apply_common(Oscillator& o, const Spec& s, Mods& m, int rate) {
    m.exp = static_cast<float>(s.get("exp", 0));
    m.lin = static_cast<float>(s.get("lin", 0));
    m.bend = static_cast<float>(s.get("bend", 0));
    m.glide = static_cast<float>(s.get("gmod", 0));
    m.unison = std::pow(2.f, static_cast<float>(s.get("unison", 0)) / 12.f); // Voice.h:574
    o.setSampleRate(static_cast<float>(rate));
    o.setPitchModExpPointer(&m.exp);
    o.setPitchModLinPointer(&m.lin);
    o.setPitchBendPointer(&m.bend);
    o.setGlidePointer(&m.glide);
    o.setUnisonDetuneFactorPointer(&m.unison);
    o.setGlide(static_cast<float>(s.get("glide", 0)));
    // Voice.h:273-316: trois multiplicateurs séparés.
    o.setOctaveMultiplier(Oscillator::pitchShiftMultiplier(static_cast<float>(s.geti("oct", 0) * 12)));
    o.setSemitoneMultiplier(Oscillator::pitchShiftMultiplier(static_cast<float>(s.geti("semi", 0))));
    o.setCentMultiplier(Oscillator::pitchShiftMultiplier(static_cast<float>(s.get("fine", 0)) * 0.01f));
}

struct Runner {
    std::function<float(float, bool)> step; // (fréquence de base, première trame)
    std::shared_ptr<void> keep;
};

void drive(Oscillator& o, float frequency, bool first) {
    if (first) o.setBaseFrequency(frequency);
    o.setGlideTargetFrequency(frequency);
}

Runner make_runner(const Spec& s, WavetableContainer& tables, int rate) {
    auto mods = std::make_shared<Mods>();
    Mods& m = *mods;
    if (s.type == "noise") { // aléatoire: comparé par spectre moyen
        auto o = std::make_shared<NoiseOscillator>();
        o->setSampleRate(static_cast<float>(rate));
        m.lfo_exp = static_cast<float>(s.get("hmod", 0));
        m.exp = static_cast<float>(s.get("lmod", 0));
        o->setHPModPointer(&m.lfo_exp);
        o->setLPModPointer(&m.exp);
        o->setFilterFreqs(static_cast<float>(s.get("lp", 18000)), static_cast<float>(s.get("hp", 80)));
        return {[=](float, bool) {
                    (void)mods;
                    return o->doNoise();
                },
                o};
    }
    if (s.type == "voice") {
        // Glide, détune d'unisson, pitch bend: AnalogOscillator sinus piloté comme la voix du plugin
        // (Voice.h:359-364 setOscBaseFreq, PluginProcessorValueChange.cpp:1100 glide ajusté au samplerate,
        // Voice.h:574 facteur d'unisson, pointeur de pitch bend en demi-tons).
        auto o = std::make_shared<AnalogOscillator>();
        apply_common(*o, s, m, rate);
        o->setGlide(static_cast<float>(std::pow(s.get("glide", 0), 44800. / rate)));
        o->setWavetableContainer(&tables);
        o->setPWMModPointer(&m.pwm);
        o->setDrift(0.0f);
        o->loadWavetables();
        o->selectWavetable(s.geti("wave", 3));
        o->reset();
        auto midi = [](int note) { return static_cast<float>(8.17579891564371 * std::pow(2.0, note / 12.0)); };
        const float last = midi(s.geti("last", s.geti("note", 69))), target = midi(s.geti("note", 69));
        return {[=](float, bool first) {
                    if (first) o->setBaseFrequency(last);
                    o->setGlideTargetFrequency(target);
                    o->update();
                    (void)mods;
                    return o->doOscillateWithSync();
                },
                o};
    }
    if (s.type == "analog") {
        auto master = std::make_shared<AnalogOscillator>();
        auto slave = std::make_shared<AnalogOscillator>();
        auto setup = [&](AnalogOscillator& o, const Spec& spec, bool is_master) {
            apply_common(o, spec, m, rate);
            o.setWavetableContainer(&tables);
            o.setPWMModPointer(&m.pwm);
            o.setDrift(0.0f);
            o.setPWMDuty(static_cast<float>(spec.get("pw", 0.5)));
            o.loadWavetables();
            o.selectWavetable(is_master ? static_cast<int>(spec.get("mwave", 0)) : spec.geti("wave", 0));
            o.reset();
        };
        const bool sync = s.geti("sync", 0) != 0;
        Spec master_spec = s;
        master_spec.kv["semi"] = s.get("msemi", 0);
        master_spec.kv["oct"] = 0;
        master_spec.kv["fine"] = 0;
        m.pwm = static_cast<float>(s.get("pwm", 0));
        setup(*slave, s, false);
        if (sync) {
            setup(*master, master_spec, true);
            slave->setSyncEnabled(true);
            slave->setSyncOscillator(master.get());
        }
        auto keep = std::make_shared<std::pair<std::shared_ptr<AnalogOscillator>, std::shared_ptr<AnalogOscillator>>>(master, slave);
        return {[=](float f, bool first) {
                    if (sync) {
                        drive(*master, f, first);
                        master->update();
                        master->doOscillateWithSync();
                    }
                    drive(*slave, f, first);
                    slave->update();
                    (void)mods;
                    return slave->doOscillateWithSync();
                },
                keep};
    }
    if (s.type == "wt2d") {
        auto o = std::make_shared<WavetableOsc2D>();
        apply_common(*o, s, m, rate);
        o->setWavetableContainer(&tables);
        o->loadWavetables();
        o->selectWavetable(s.geti("wt", 1) - 1);
        m.pos = static_cast<float>(s.get("pmod", 0));
        o->setPosModPointer(&m.pos);
        o->setPosition(static_cast<float>(s.get("pos", 0)));
        o->setPosModAmount(static_cast<float>(s.get("pamt", 0)));
        o->setPosModValue(static_cast<float>(s.get("psrc", 0)));
        const bool sync = s.geti("sync", 0) != 0;
        auto master = std::make_shared<AnalogOscillator>();
        Spec mspec = s;
        mspec.kv["semi"] = s.get("msemi", 0);
        mspec.kv["oct"] = 0;
        mspec.kv["fine"] = 0;
        if (sync) {
            apply_common(*master, mspec, m, rate);
            master->setWavetableContainer(&tables);
            master->setPWMModPointer(&m.pwm);
            master->loadWavetables();
            master->selectWavetable(0);
            master->reset();
            o->setSyncEnabled(true);
            o->setSyncOscillator(master.get());
        }
        o->reset();
        return {[=](float f, bool first) {
                    if (sync) {
                        drive(*master, f, first);
                        master->update();
                        master->doOscillateWithSync();
                    }
                    drive(*o, f, first);
                    o->update();
                    (void)mods;
                    return o->doOscillateWithSync();
                },
                std::make_shared<std::pair<std::shared_ptr<WavetableOsc2D>, std::shared_ptr<AnalogOscillator>>>(o, master)};
    }
    if (s.type == "multi") {
        auto o = std::make_shared<MultiOscillator>();
        apply_common(*o, s, m, rate);
        o->setWavetableContainer(&tables);
        o->loadWavetables();
        o->selectWavetable(s.geti("wt", 1) - 1);
        m.pos = static_cast<float>(s.get("pmod", 0));
        m.detune = static_cast<float>(s.get("dmod", 0));
        m.spread = static_cast<float>(s.get("smod", 0));
        o->setPosModPointer(&m.pos);
        o->setDetuneModPointer(&m.detune);
        o->setSpreadModPointer(&m.spread);
        o->setPosition(static_cast<float>(s.get("pos", 0)));
        o->setDetune(static_cast<float>(s.get("detune", 0.2)));
        o->setWavetableMultiSpread(static_cast<float>(s.get("spread", 0)));
        o->reset();
        if (s.geti("randphase", 0)) {
            std::srand(static_cast<unsigned>(s.get("seed", 0)));
            o->randomizePhase();
        }
        return {[=](float f, bool first) {
                    drive(*o, f, first);
                    o->update();
                    (void)mods;
                    return o->doOscillate();
                },
                o};
    }
    if (s.type == "vector") {
        auto o = std::make_shared<VectorOscillator>();
        apply_common(*o, s, m, rate);
        o->setWavetableContainer(&tables);
        o->loadWavetables();
        o->selectWavetableByMappingVector(s.geti("a", 101), 0);
        o->selectWavetableByMappingVector(s.geti("b", 102), 1);
        o->selectWavetableByMappingVector(s.geti("c", 103), 2);
        o->selectWavetableByMappingVector(s.geti("d", 104), 3);
        m.x = static_cast<float>(s.get("xmod", 0));
        m.y = static_cast<float>(s.get("ymod", 0));
        o->setXModPointer(&m.x);
        o->setYModPointer(&m.y);
        o->setXYPad(static_cast<float>(s.get("x", 0)), static_cast<float>(s.get("y", 0)));
        const bool sync = s.geti("sync", 0) != 0;
        auto master = std::make_shared<AnalogOscillator>();
        Spec mspec = s;
        mspec.kv["semi"] = s.get("msemi", 0);
        mspec.kv["oct"] = 0;
        mspec.kv["fine"] = 0;
        if (sync) {
            apply_common(*master, mspec, m, rate);
            master->setWavetableContainer(&tables);
            master->setPWMModPointer(&m.pwm);
            master->loadWavetables();
            master->selectWavetable(0);
            master->reset();
            o->setSyncEnabled(true);
            o->setSyncOscillator(master.get());
        }
        o->reset();
        return {[=](float f, bool first) {
                    if (sync) {
                        drive(*master, f, first);
                        master->update();
                        master->doOscillateWithSync();
                    }
                    drive(*o, f, first);
                    o->update();
                    (void)mods;
                    return o->doOscillateWithSync();
                },
                std::make_shared<std::pair<std::shared_ptr<VectorOscillator>, std::shared_ptr<AnalogOscillator>>>(o, master)};
    }
    if (s.type == "chiptune") {
        auto o = std::make_shared<ChiptuneOscillator>();
        apply_common(*o, s, m, rate);
        o->setWavetableContainer(&tables);
        o->loadWavetables();
        o->selectWavetable(o->wavetableMappingChiptune(s.geti("wave", 1)));
        m.arp = static_cast<float>(s.get("amod", 0));
        o->setArpSpeedModPointer(&m.arp);
        o->setArpSpeed(static_cast<float>(s.get("aspeed", 20)));
        o->setArpSemitone(0, s.geti("s1", 0));
        o->setArpSemitone(1, s.geti("s2", 12));
        o->setArpSemitone(2, s.geti("s3", 7));
        o->setArpStepThreeOn(s.geti("s3on", 0) != 0);
        o->setArpEnabled(s.geti("arp", 0) != 0);
        o->setNoiseEnabled(s.geti("noise", 0) != 0);
        const bool sync = s.geti("sync", 0) != 0;
        auto master = std::make_shared<AnalogOscillator>();
        Spec mspec = s;
        mspec.kv["semi"] = s.get("msemi", 0);
        mspec.kv["oct"] = 0;
        mspec.kv["fine"] = 0;
        if (sync) {
            apply_common(*master, mspec, m, rate);
            master->setWavetableContainer(&tables);
            master->setPWMModPointer(&m.pwm);
            master->loadWavetables();
            master->selectWavetable(0);
            master->reset();
            o->setSyncEnabled(true);
            o->setSyncOscillator(master.get());
        }
        o->reset();
        return {[=](float f, bool first) {
                    if (sync) {
                        drive(*master, f, first);
                        master->update();
                        master->doOscillateWithSync();
                    }
                    drive(*o, f, first);
                    o->update();
                    (void)mods;
                    return o->doOscillateWithSync();
                },
                std::make_shared<std::pair<std::shared_ptr<ChiptuneOscillator>, std::shared_ptr<AnalogOscillator>>>(o, master)};
    }
    if (s.type == "fm") {
        auto o = std::make_shared<FMOscillator>();
        apply_common(*o, s, m, rate);
        o->setWavetableContainer(&tables);
        o->loadWavetables();
        o->m_carrier_osc.selectWavetable(o->m_carrier_osc.wavetableMappingFM(s.geti("cwave", 1)));
        o->m_modulator_osc.selectWavetable(o->m_modulator_osc.wavetableMappingFM(s.geti("mwave", 1)));
        m.fm = static_cast<float>(s.get("amod", 0));
        m.carrier = static_cast<float>(s.get("crmod", 0));
        m.modulator = static_cast<float>(s.get("mrmod", 0));
        o->setFMModPointer(&m.fm);
        o->setCarrierRatioModPointer(&m.carrier);
        o->setModulatorRatioModPointer(&m.modulator);
        o->setFMAmount(static_cast<float>(s.get("amount", 0)));
        o->setRatio(s.geti("cr", 1), s.geti("mr", 1));
        o->setFMExponential(s.geti("expfm", 0) != 0);
        o->reset();
        return {[=](float f, bool first) {
                    drive(*o, f, first);
                    o->update();
                    (void)mods;
                    return o->doOscillate();
                },
                o};
    }
    if (s.type == "pm") {
        auto o = std::make_shared<PMOscillator>();
        apply_common(*o, s, m, rate);
        o->setWavetableContainer(&tables);
        o->loadWavetables();
        o->m_carrier_osc.selectWavetable(o->m_carrier_osc.wavetableMappingFM(s.geti("cwave", 1)));
        o->m_modulator_osc.selectWavetable(o->m_modulator_osc.wavetableMappingFM(s.geti("mwave", 1)));
        m.fm = static_cast<float>(s.get("amod", 0));
        m.carrier = static_cast<float>(s.get("crmod", 0));
        m.modulator = static_cast<float>(s.get("mrmod", 0));
        o->setPMModPointer(&m.fm);
        o->setCarrierRatioModPointer(&m.carrier);
        o->setModulatorRatioModPointer(&m.modulator);
        o->setPMAmount(static_cast<float>(s.get("amount", 0)));
        o->setRatio(s.geti("cr", 1), s.geti("mr", 1));
        o->reset();
        return {[=](float f, bool first) {
                    drive(*o, f, first);
                    o->update();
                    (void)mods;
                    return o->doOscillate();
                },
                o};
    }
    if (s.type == "wt1d") { // wavedraw / chipdraw / specdraw: WavetableOsc1D + tables dessinées
        auto o = std::make_shared<WavetableOsc1D>();
        apply_common(*o, s, m, rate);
        o->setWavetableContainer(&tables);
        const int kind = s.geti("kind", 0), index = s.geti("idx", 0);
        if (kind == 0) o->loadWavedrawTables(index);
        else if (kind == 1) o->loadChipdrawTables(index);
        else o->loadSpecdrawTables(index);
        o->reset();
        return {[=](float f, bool first) {
                    drive(*o, f, first);
                    o->update();
                    (void)mods;
                    return o->doOscillateWithSync();
                },
                o};
    }
    if (s.type == "lfo") {
        auto o = std::make_shared<LFO>();
        o->setSampleRate(static_cast<float>(rate));
        o->setWavetableContainer(&tables);
        o->loadWavetables();
        m.lfo_exp = static_cast<float>(s.get("mod", 0));
        o->setPitchModExpPointer(&m.lfo_exp);
        // PluginProcessorValueChange.cpp:1213-1256: sélection par identifiant de menu.
        const float wave = static_cast<float>(s.get("wave", 0));
        if (wave < 6.5f) {
            o->selectWavetable(wave);
            o->setSHActive(false);
        } else if (wave > 7.5f) {
            o->selectWavetable(static_cast<int>(wave) - 1);
            o->setSHActive(false);
        } else {
            o->setSHActive(true);
        }
        const bool sync = s.geti("sync", 0) != 0;
        static const float denominators[] = {1.f, 2.f, 4.f, 8.f, 12.f, 16.f, 24.f, 32.f, 48.f};
        o->setSynctimeNumerator(static_cast<float>(s.geti("num", 2) + 1));
        const int den = s.geti("den", 5);
        o->setSynctimeDenominator(den >= 0 && den <= 8 ? denominators[den] : 16.f);
        if (sync) o->setFreqBPM(static_cast<float>(s.get("bpm", 120)));
        else o->setBaseFrequency(static_cast<float>(s.get("freq", 1)));
        o->reset();
        return {[=](float, bool) {
                    o->update();
                    (void)mods;
                    return o->doOscillate();
                },
                o};
    }
    throw std::runtime_error("type de scénario inconnu: " + s.type);
}

void run_scenario(const Spec& spec, WavetableContainer& tables, int rate, const std::filesystem::path& stimuli,
                  const std::filesystem::path& output) {
    std::ifstream input(stimuli / spec.stim);
    std::ofstream result(output / (spec.name + ".csv"));
    if (!input || !result) throw std::runtime_error("CSV impossible: " + spec.name);
    result << "frame,input,output\n" << std::setprecision(17);
    auto runner = make_runner(spec, tables, rate);
    std::string line;
    std::getline(input, line);
    bool first = true;
    while (std::getline(input, line)) {
        const auto comma = line.find(',');
        const float frequency = std::stof(line.substr(comma + 1));
        const float value = runner.step(frequency, first);
        first = false;
        result << line.substr(0, comma) << ',' << frequency << ',' << static_cast<double>(value) << '\n';
    }
}

std::uint64_t fnv(const float* data, int count) {
    std::uint64_t hash = 1469598103934665603ull;
    const auto* bytes = reinterpret_cast<const unsigned char*>(data);
    for (std::size_t i = 0; i < static_cast<std::size_t>(count) * sizeof(float); ++i) {
        hash ^= bytes[i];
        hash *= 1099511628211ull;
    }
    return hash;
}

struct Probe2D : WavetableOsc2D {
    const float* table(int wt, int sub_table, int morph) const {
        return m_wavetable_pointers_2D[wt][sub_table][morph];
    }
};

void dump_static(const std::filesystem::path& output, WavetableContainer& tables) {
    {
        std::ofstream data(output / "tables.f32", std::ios::binary);
        for (int wt = 0; wt < NUMBER_OF_WAVETABLES; ++wt)
            for (int sub = 0; sub < SUBTABLES_PER_WAVETABLE; ++sub)
                data.write(reinterpret_cast<const char*>(getOneSubTable(wt, sub)), WAVETABLE_LENGTH * sizeof(float));
        std::ofstream lfo(output / "lfo.f32", std::ios::binary);
        for (int table = 0; table < NUMBER_OF_LFOTABLES; ++table)
            lfo.write(reinterpret_cast<const char*>(getOneLFOTable(table)), WAVETABLE_LENGTH * sizeof(float));
    }

    WavetableOsc1D osc;
    std::ofstream mappings(output / "mappings.txt");
    for (int code = 0; code < 1000; ++code)
        mappings << code << ' ' << osc.wavetableMappingVector(code) << ' ' << osc.wavetableMappingFM(code) << ' '
                 << osc.wavetableMappingChiptune(code) << '\n';

    // Identité de chaque sous-table 2D: indice du wavetable 1D dont la sous-table 0 est le même pointeur.
    auto probe = std::make_unique<Probe2D>();
    probe->setWavetableContainer(&tables);
    probe->loadWavetables();
    std::ofstream morph(output / "morph.txt");
    for (int wt = 0; wt < 35; ++wt) { // 35 wavetables 2D définis (WavetableOsc2D.cpp:25-201)
        morph << wt;
        for (int k = 0; k < TABLES_PER_2D_WT; ++k) {
            int found = -1;
            const float* pointer = probe->table(wt, 0, k);
            for (int i = 0; i < NUMBER_OF_WAVETABLES; ++i)
                if (pointer == getOneSubTable(i, 0)) found = i;
            morph << ' ' << found;
        }
        morph << '\n';
    }
}

void dump_draw(const std::filesystem::path& output, WavetableContainer& tables) {
    const char* names[] = {"wave", "chip", "spec"};
    for (int kind = 0; kind < 3; ++kind)
        for (int osc = 0; osc < 3; ++osc) {
            float** pointers = kind == 0 ? tables.getWavedrawPointer(osc)
                               : kind == 1 ? tables.getChipdrawPointer(osc)
                                           : tables.getSpecdrawPointer(osc);
            std::ofstream file(output / (std::string("draw_") + names[kind] + std::to_string(osc) + ".f32"), std::ios::binary);
            for (int sub = 0; sub < SUBTABLES_PER_WAVETABLE; ++sub)
                file.write(reinterpret_cast<const char*>(pointers[sub]), WAVETABLE_LENGTH * sizeof(float));
        }
}

} // namespace

int main(int argc, char** argv) {
    try {
        if (argc != 5) throw std::runtime_error("Usage: odin_osc_modules SPEC STIMULI OUT RATE");
        const int rate = std::stoi(argv[4]);
        if (rate < 8000) throw std::runtime_error("RATE >= 8000 requis");
        const std::filesystem::path stimuli(argv[2]), output(argv[3]);
        std::filesystem::create_directories(output);
        auto specs = read_specs(argv[1]);
        std::srand(0);
        auto tables = std::make_unique<WavetableContainer>();
        tables->loadWavetablesFromConstData();
        // Le seed des dessins est global au fichier de scénarios (ligne "# drawseed N").
        std::uint32_t seed = 1;
        {
            std::ifstream header(argv[1]);
            std::string line;
            while (std::getline(header, line))
                if (line.rfind("# drawseed ", 0) == 0) seed = static_cast<std::uint32_t>(std::stoul(line.substr(11)));
        }
        make_draw_tables(*tables, seed);
        dump_static(output, *tables);
        dump_draw(output, *tables);
        for (const auto& spec : specs) {
            std::srand(static_cast<unsigned>(spec.get("seed", 0)));
            run_scenario(spec, *tables, rate, stimuli, output);
        }
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
