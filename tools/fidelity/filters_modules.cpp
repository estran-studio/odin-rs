// GPL-3.0-or-later. Banc filtres: vrais modules audio amont Diode/Korg35/SEM/Comb/Formant/RingMod
// et chaîne d'entrée OdinFilterBase::update() pilotée comme PluginProcessorProcess.cpp.
// Usage: odin_filters_modules CASES STIMULI OUT RATE
// CASES: une ligne par cas « nom genre clé=valeur ... ». Voir scripts/compare_filters.py.
#include "audio/Filters/BiquadAllpass.h"
#include "audio/Filters/CombFilter.h"
#include "audio/Filters/DiodeFilter.h"
#include "audio/Filters/FormantFilter.h"
#include "audio/Filters/Korg35Filter.h"
#include "audio/Filters/LadderFilter.h"
#include "audio/Filters/SEMFilter12.h"
#include "audio/FX/RingModulator.h"
#include "audio/Oscillators/WavetableContainer.h"
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

namespace {
struct Case {
    std::string name, kind;
    std::map<std::string, double> values;
    double get(const char* key, double fallback) const {
        const auto it = values.find(key);
        return it == values.end() ? fallback : it->second;
    }
};

using SampleFn = std::function<double(float)>;

// Mêmes variables que les pointeurs de modulation de PluginProcessorSetModPointers.cpp.
struct Mods {
    float freq = 0, res = 0, sat = 0, env = 0, vel = 0, kbd = 0, transition = 0, ring = 0;
};

void capture(const std::filesystem::path& stimulus, const std::filesystem::path& output, const SampleFn& process) {
    std::ifstream input(stimulus);
    std::ofstream result(output);
    if (!input || !result) throw std::runtime_error("Ouverture CSV impossible: " + output.string());
    result << "frame,input,output\n" << std::setprecision(17);
    std::string line;
    std::getline(input, line);
    while (std::getline(input, line)) {
        const auto comma = line.find(',');
        if (comma == std::string::npos) throw std::runtime_error("CSV stimulus incorrect");
        const float sample = std::stof(line.substr(comma + 1));
        const double value = process(sample);
        if (!std::isfinite(value)) throw std::runtime_error("Module C++ non fini: " + output.string());
        result << line.substr(0, comma) << ',' << sample << ',' << value << '\n';
    }
    if (!result) throw std::runtime_error("Écriture CSV impossible");
}

void init_base(OdinFilterBase& filter, Mods& m, const Case& c, int rate) {
    filter.setSampleRate(rate);
    filter.setFreqModPointer(&m.freq);
    filter.setResModPointer(&m.res);
    filter.setVelModPointer(&m.vel);
    filter.setKbdModPointer(&m.kbd);
    filter.setSaturationModPointer(&m.sat);
    filter.setEnvModPointer(&m.env);
    filter.m_MIDI_note = (int)c.get("note", 0);
    filter.m_MIDI_velocity = (int)c.get("velocity", 0);
    filter.m_kbd_mod_amount = (float)c.get("kbd", 0);
    filter.m_vel_mod_amount = (float)c.get("velamt", 0);
    filter.m_env_mod_amount = (float)c.get("envamt", 0);
    filter.m_env_value = (float)c.get("env", 0);
}

// Gain de filtre modulé, PluginProcessorProcess.cpp:305-328; `gain` est la valeur lissée.
float gain_law(float gain, float modulation) {
    float modded = gain;
    if (modulation) {
        if (modulation < 0.f) {
            modded = gain * (1.f + modulation);
            modded = modded < 0 ? 0 : modded;
        } else if (gain > MINUS_12_dB_GAIN) {
            modded *= pow(PLUS_12_dB_GAIN, modulation);
            modded = modded > PLUS_12_dB_GAIN ? PLUS_12_dB_GAIN : modded;
        } else {
            modded += (1.f - modded) * modulation;
            modded = modded > PLUS_12_dB_GAIN ? PLUS_12_dB_GAIN : modded;
        }
    }
    return modded;
}

// Un module nu: les réglages directs de la classe, sans chaîne preset.
SampleFn make_module(const Case& c, int rate, WavetableContainer& tables) {
    auto mods = std::make_shared<Mods>();
    const double freq = c.get("freq", 1000.0), res = c.get("res", 0.25), od = c.get("od", 0.0);
    if (c.kind == "diode" || c.kind == "korg" || c.kind == "sem") {
        std::shared_ptr<OdinFilterBase> filter;
        if (c.kind == "diode") filter = std::make_shared<DiodeFilter>();
        else if (c.kind == "korg") {
            auto korg = std::make_shared<Korg35Filter>();
            korg->setFilterType(c.get("lp", 1) != 0);
            filter = korg;
        } else {
            auto sem = std::make_shared<SEMFilter12>();
            sem->m_transition = c.get("trans", -1);
            sem->setTransitionModPointer(&mods->transition);
            filter = sem;
        }
        init_base(*filter, *mods, c, rate);
        filter->m_freq_base = freq;
        filter->m_overdrive = (float)od;
        filter->setResControl(res);
        filter->reset();
        return [=, keep = mods](float x) {
            filter->update();
            return filter->doFilter(x);
        };
    }
    if (c.kind == "formant") {
        auto filter = std::make_shared<FormantFilter>();
        init_base(*filter, *mods, c, rate);
        filter->setTransitionModPointer(&mods->transition);
        filter->setVowelLeft((int)c.get("vl", 0));
        filter->setVowelRight((int)c.get("vr", 1));
        filter->setTransition((float)c.get("trans", 0));
        filter->reset();
        return [=, keep = mods](float x) {
            filter->update();
            return filter->doFilter(x);
        };
    }
    if (c.kind == "comb") {
        auto filter = std::make_shared<CombFilter>();
        filter->setSampleRate((float)rate);
        filter->setFreqModPointer(&mods->freq);
        filter->setResModPointer(&mods->res);
        filter->setEnvModPointer(&mods->env);
        filter->setVelModPointer(&mods->vel);
        filter->setKbdModPointer(&mods->kbd);
        filter->m_MIDI_note = (int)c.get("note", 0);
        filter->m_MIDI_velocity = (int)c.get("velocity", 0);
        filter->m_kbd_mod_amount = (float)c.get("kbd", 0);
        filter->m_vel_mod_amount = (float)c.get("velamt", 0);
        filter->m_env_mod_amount = (float)c.get("envamt", 0);
        filter->m_env_value = (float)c.get("env", 0);
        filter->setPositive(c.get("pos", 1) != 0);
        filter->setResonance((float)res);
        filter->setCombFreq((float)freq);
        if (c.get("noreset", 0) == 0) filter->reset();
        return [=, keep = mods](float x) {
            filter->setCombFreq((float)freq);
            return filter->doFilter(x);
        };
    }
    if (c.kind == "allpass") {
        auto filter = std::make_shared<BiquadAllpass>();
        filter->setSampleRate((float)rate);
        filter->setRadius((float)c.get("radius", 1.32));
        filter->setFrequency((float)freq);
        filter->reset();
        return [=](float x) { return filter->doFilter(x); };
    }
    if (c.kind == "ringmod") {
        auto ring = std::make_shared<RingModulator>();
        ring->setSampleRate((float)rate);
        ring->setWavetableContainer(&tables);
        ring->loadWavetables();
        ring->selectWavetable(0);
        ring->setPitchModExpPointer(&mods->freq);
        ring->setVelModPointer(&mods->vel);
        ring->setKbdModPointer(&mods->kbd);
        ring->setEnvModPointer(&mods->env);
        ring->setRingModAmountModPointer(&mods->ring);
        ring->setAmount((float)c.get("amount", 1));
        ring->m_MIDI_note = (int)c.get("note", 0);
        ring->m_MIDI_velocity = (int)c.get("velocity", 0);
        ring->m_kbd_mod_amount = (float)c.get("kbd", 0);
        ring->m_vel_mod_amount = (float)c.get("velamt", 0);
        ring->m_env_mod_amount = (float)c.get("envamt", 0);
        ring->m_env_value = (float)c.get("env", 0);
        ring->reset();
        return [=, keep = mods](float x) {
            ring->setBaseFrequency((float)freq);
            ring->setGlideTargetFrequency((float)freq);
            ring->update();
            return ring->doRingModulator(x);
        };
    }
    throw std::runtime_error("Genre de cas inconnu: " + c.kind);
}

// Chaîne preset complète, valeurs stationnaires. `type` suit FilterType du Rust (== menu Odin 2+).
SampleFn make_preset(const Case& c, int rate, WavetableContainer& tables) {
    auto mods = std::make_shared<Mods>();
    mods->freq = (float)c.get("mfreq", 0);
    mods->res = (float)c.get("mres", 0);
    mods->sat = (float)c.get("msat", 0);
    mods->env = (float)c.get("menv", 0);
    mods->vel = (float)c.get("mvel", 0);
    mods->kbd = (float)c.get("mkbd", 0);
    mods->transition = (float)c.get("mtrans", 0);
    mods->ring = (float)c.get("mring", 0);
    const int type = (int)c.get("type", 2);
    const double freq = c.get("freq", 1000), res = c.get("res", 0.25), sat = c.get("sat", 0);
    const float gain_mod = (float)c.get("mgain", 0);
    // PluginProcessorProcess.cpp:70-76: lissage de fréquence (0.998f) et de gain (0.995f), départ à la cible.
    struct Smooth { float freq, gain; };
    const float freq_control = (float)freq;
    const float gain_control = juce::Decibels::decibelsToGain((float)c.get("gain", 0), -59.99f);
    auto smooth = std::make_shared<Smooth>(Smooth{freq_control, gain_control});
    auto advance = [smooth, freq_control, gain_control]() {
        smooth->gain = smooth->gain * GAIN_SMOOTHIN_FACTOR + (1.f - GAIN_SMOOTHIN_FACTOR) * gain_control;
        smooth->freq = smooth->freq * FILTER_FREQ_SMOOTHING_FACTOR + (1.f - FILTER_FREQ_SMOOTHING_FACTOR) * freq_control;
    };
    const bool is_ladder = type >= 2 && type <= 7;
    std::shared_ptr<OdinFilterBase> filter;
    auto setup = [&](OdinFilterBase& f) {
        init_base(f, *mods, c, rate);
        f.m_freq_base = freq;
        f.m_overdrive = (float)sat * 2; // PluginProcessorValueChange.cpp:561
        f.setResControl(res);
    };
    if (is_ladder) {
        auto f = std::make_shared<LadderFilter>();
        setup(*f);
        f->setFilterType(type - 2);
        f->reset();
        filter = f;
    } else if (type == 15) {
        auto f = std::make_shared<SEMFilter12>();
        setup(*f);
        f->m_transition = c.get("trans", -1);
        f->setTransitionModPointer(&mods->transition);
        f->reset();
        filter = f;
    } else if (type == 19) {
        auto f = std::make_shared<DiodeFilter>();
        setup(*f);
        f->reset();
        filter = f;
    } else if (type == 24 || type == 25) {
        auto f = std::make_shared<Korg35Filter>();
        setup(*f);
        f->setFilterType(type == 24);
        f->reset();
        filter = f;
    } else if (type == 35) {
        auto f = std::make_shared<FormantFilter>();
        init_base(*f, *mods, c, rate);
        // Voice::setFilterMIDIValues n'affecte jamais FormantFilter::m_MIDI_velocity: il reste à 0.
        f->m_MIDI_velocity = 0.f;
        f->setTransitionModPointer(&mods->transition);
        f->setVowelLeft((int)c.get("vl", 0));
        f->setVowelRight((int)c.get("vr", 1));
        f->setTransition((float)c.get("ftrans", 0));
        f->reset();
        filter = f;
    } else if (type == 30) {
        auto comb = std::make_shared<CombFilter>();
        comb->setSampleRate((float)rate);
        comb->setFreqModPointer(&mods->freq);
        comb->setResModPointer(&mods->res);
        comb->setEnvModPointer(&mods->env);
        comb->setVelModPointer(&mods->vel);
        comb->setKbdModPointer(&mods->kbd);
        comb->m_MIDI_note = (int)c.get("note", 0);
        comb->m_MIDI_velocity = (int)c.get("velocity", 0);
        comb->m_kbd_mod_amount = (float)c.get("kbd", 0);
        comb->m_vel_mod_amount = (float)c.get("velamt", 0);
        comb->m_env_mod_amount = (float)c.get("envamt", 0);
        comb->m_env_value = (float)c.get("env", 0);
        // PluginProcessorValueChange.cpp:1486: setPositive(!polarity)
        comb->setPositive(!((bool)c.get("pol", 0)));
        comb->setResonance((float)res);
        comb->setCombFreq((float)freq);
        comb->reset();
        return [=, keep = mods](float x) {
            advance();
            comb->setCombFreq(smooth->freq);
            return comb->doFilter(x) * gain_law(smooth->gain, gain_mod);
        };
    } else if (type == 40) {
        auto ring = std::make_shared<RingModulator>();
        ring->setSampleRate((float)rate);
        ring->setWavetableContainer(&tables);
        ring->loadWavetables();
        ring->selectWavetable(0);
        ring->setPitchModExpPointer(&mods->freq);
        ring->setVelModPointer(&mods->vel);
        ring->setKbdModPointer(&mods->kbd);
        ring->setEnvModPointer(&mods->env);
        ring->setRingModAmountModPointer(&mods->ring);
        ring->setAmount((float)c.get("ring", 1));
        ring->m_MIDI_note = (int)c.get("note", 0);
        ring->m_MIDI_velocity = (int)c.get("velocity", 0);
        ring->m_kbd_mod_amount = (float)c.get("kbd", 0);
        ring->m_vel_mod_amount = (float)c.get("velamt", 0);
        ring->m_env_mod_amount = (float)c.get("envamt", 0);
        ring->m_env_value = (float)c.get("env", 0);
        ring->reset();
        return [=, keep = mods](float x) {
            advance();
            ring->setBaseFrequency(smooth->freq);
            ring->setGlideTargetFrequency(smooth->freq);
            ring->update();
            return ring->doRingModulator(x) * gain_law(smooth->gain, gain_mod);
        };
    } else {
        throw std::runtime_error("type preset inconnu");
    }
    return [=, keep = mods](float x) {
        advance();
        filter->m_freq_base = smooth->freq;
        filter->update();
        return filter->doFilter(x) * gain_law(smooth->gain, gain_mod);
    };
}
} // namespace

int main(int argc, char** argv) {
    try {
        if (argc != 5) throw std::runtime_error("Usage: odin_filters_modules CASES STIMULI OUT RATE");
        const std::filesystem::path cases_path(argv[1]), stimuli(argv[2]), output(argv[3]);
        const int rate = std::stoi(argv[4]);
        std::filesystem::create_directories(output);
        auto tables = std::make_unique<WavetableContainer>();
        tables->loadWavetablesFromConstData();
        std::ifstream cases_file(cases_path);
        if (!cases_file) throw std::runtime_error("CASES illisible");
        std::string line;
        while (std::getline(cases_file, line)) {
            std::istringstream words(line);
            Case c;
            if (!(words >> c.name >> c.kind)) continue;
            std::string token;
            while (words >> token) {
                const auto eq = token.find('=');
                if (eq == std::string::npos) throw std::runtime_error("jeton invalide: " + token);
                c.values[token.substr(0, eq)] = std::stod(token.substr(eq + 1));
            }
            for (const auto* scenario : {"impulse", "sine", "sweep"}) {
                SampleFn process = c.kind == "preset" ? make_preset(c, rate, *tables) : make_module(c, rate, *tables);
                capture(stimuli / (std::string(scenario) + ".csv"),
                        output / (c.name + "_" + scenario + ".csv"), process);
            }
        }
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
