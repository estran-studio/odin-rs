// GPL-3.0-or-later. Modules audio amont inchangés, avec leur vrai JUCE.
#include "audio/Filters/LadderFilter.h"
#include "audio/Filters/VAOnePoleFilter.h"
#include "audio/FX/OversamplingDistortion.h"
#include "audio/Oscillators/AnalogOscillator.h"
#include <filesystem>
#include <fstream>
#include <functional>
#include <iomanip>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>

namespace {
void capture(const std::filesystem::path& stimulus, const std::filesystem::path& output,
             const std::function<double(float)>& process) {
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
        if (!std::isfinite(value)) throw std::runtime_error("Module C++ non fini");
        result << line.substr(0, comma) << ',' << sample << ',' << value << '\n';
    }
    if (!result) throw std::runtime_error("Écriture CSV impossible");
}
void init_filter(OdinFilterBase& filter, float& zero, int rate) {
    filter.setSampleRate(rate);
    filter.setFreqModPointer(&zero);
    filter.setResModPointer(&zero);
    filter.setVelModPointer(&zero);
    filter.setKbdModPointer(&zero);
    filter.setSaturationModPointer(&zero);
    filter.setEnvModPointer(&zero);
    filter.m_freq_base = 1000.0;
    filter.m_overdrive = 0.0f;
    filter.setResControl(0.25);
    filter.reset();
}
}
int main(int argc, char** argv) {
    try {
        if (argc != 4) throw std::runtime_error("Usage: odin_audio_modules STIMULI OUT RATE");
        const std::filesystem::path stimuli(argv[1]), output(argv[2]);
        const int rate = std::stoi(argv[3]);
        if (rate < 8000) throw std::runtime_error("RATE >= 8000 requis");
        std::filesystem::create_directories(output);
        float zero = 0.0f;
        const char* ladder_names[] = {"lp24", "lp12", "bp24", "bp12", "hp24", "hp12"};
        for (const auto* scenario : {"impulse", "sine", "sweep"}) {
            const auto stimulus = stimuli / (std::string(scenario) + ".csv");
            for (int mode = 0; mode < 6; ++mode) {
                LadderFilter filter;
                init_filter(filter, zero, rate);
                filter.setFilterType(mode);
                filter.update();
                capture(stimulus, output / (std::string("ladder_") + ladder_names[mode] + "_" + scenario + ".csv"),
                        [&](float input) { filter.update(); return filter.doFilter(input); });
            }
            for (int hp = 0; hp < 2; ++hp) {
                VAOnePoleFilter filter;
                init_filter(filter, zero, rate);
                if (hp) filter.setHP(); else filter.setLP();
                filter.update();
                capture(stimulus, output / (std::string("onepole_") + (hp ? "hp_" : "lp_") + scenario + ".csv"),
                        [&](float input) { return filter.doFilter(input); });
            }
            const char* algorithms[] = {"clamp", "fold", "zero", "sine", "cube"};
            for (int algorithm = 1; algorithm <= 5; ++algorithm) {
                OversamplingDistortion distortion;
                distortion.setThresholdModPointer(&zero);
                distortion.setDryWetModPointer(&zero);
                distortion.setAlgorithm(algorithm);
                distortion.setThreshold(0.5f);
                distortion.setDryWet(0.75f);
                distortion.reset();
                capture(stimulus, output / (std::string("distortion_") + algorithms[algorithm - 1] + "_" + scenario + ".csv"),
                        [&](float input) { return distortion.doDistortion(input); });
            }
        }
        std::srand(0);
        auto tables = std::make_unique<WavetableContainer>();
        tables->loadWavetablesFromConstData();
        const char* wave_names[] = {"saw", "pulse", "triangle", "sine"};
        for (int waveform = 0; waveform < 4; ++waveform) {
            for (const auto* scenario : {"tone", "sweep"}) {
                AnalogOscillator oscillator;
                oscillator.setSampleRate(static_cast<float>(rate));
                oscillator.setWavetableContainer(tables.get());
                oscillator.setPitchModExpPointer(&zero);
                oscillator.setPitchModLinPointer(&zero);
                oscillator.setPitchBendPointer(&zero);
                oscillator.setGlidePointer(&zero);
                oscillator.setPWMModPointer(&zero);
                oscillator.setGlide(0.0f);
                oscillator.setDrift(0.0f);
                oscillator.setPWMDuty(0.5f);
                oscillator.setSmoothValues();
                oscillator.loadWavetables();
                oscillator.selectWavetable(waveform);
                oscillator.reset();
                capture(stimuli / (std::string("frequency_") + scenario + ".csv"),
                        output / (std::string("analog_") + wave_names[waveform] + "_" + scenario + ".csv"),
                        [&](float frequency) {
                            oscillator.setBaseFrequency(frequency);
                            oscillator.setGlideTargetFrequency(frequency);
                            oscillator.update();
                            return oscillator.doOscillateWithSync();
                        });
            }
        }
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
