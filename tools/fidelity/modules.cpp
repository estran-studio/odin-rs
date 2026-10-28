// GPL-3.0-or-later. Pilote uniquement: les calculs viennent des sources amont.
#include "Source/audio/ADSR.h"
#include "Source/audio/Filters/BiquadFilter.h"
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>

int main(int argc, char** argv) {
    try {
        if (argc != 4) throw std::runtime_error("Usage: odin_modules STIMULI OUT RATE");
        const std::filesystem::path stimuli(argv[1]), output(argv[2]);
        const int rate = std::stoi(argv[3]);
        if (rate < 8000) throw std::runtime_error("RATE >= 8000 requis");
        std::filesystem::create_directories(output);
        for (const auto* scenario : {"impulse", "sine", "sweep"}) {
            std::ifstream input(stimuli / (std::string(scenario) + ".csv"));
            std::ofstream result(output / (std::string("biquad_") + scenario + ".csv"));
            if (!input || !result) throw std::runtime_error("Ouverture CSV impossible");
            result << "frame,input,output\n" << std::setprecision(17);
            BiquadFilter filter;
            filter.setSampleRate(static_cast<float>(rate));
            filter.setFilterCoefficients(-0.5, 0.125, 0.125, 0.25, 0.125);
            filter.reset();
            std::string line;
            std::getline(input, line);
            while (std::getline(input, line)) {
                const auto comma = line.find(',');
                if (comma == std::string::npos) throw std::runtime_error("CSV stimulus incorrect");
                const float sample = std::stof(line.substr(comma + 1));
                result << line.substr(0, comma) << ',' << sample << ',' << filter.doFilter(sample) << '\n';
            }
            if (!result) throw std::runtime_error("Écriture CSV impossible");
        }
        ADSREnvelope envelope;
        float zero = 0.0f;
        bool voice = true, manager = true;
        envelope.setAttackModPointer(&zero);
        envelope.setDecayModPointer(&zero);
        envelope.setSustainModPointer(&zero);
        envelope.setReleaseModPointer(&zero);
        envelope.setEnvelopeEndPointers(&voice, &manager);
        envelope.setSampleRate(static_cast<float>(rate));
        envelope.setAttack(0.01f);
        envelope.setDecay(0.1f);
        envelope.setSustain(0.4f);
        envelope.setRelease(0.2f);
        envelope.restartEnvelopeRetrig();
        std::ofstream result(output / "adsr_gate.csv");
        if (!result) throw std::runtime_error("Ouverture CSV impossible");
        result << "frame,input,output\n" << std::setprecision(17);
        for (int frame = 0; frame < rate * 2; ++frame) {
            if (frame == rate * 3 / 4) envelope.startRelease();
            result << frame << ',' << (frame < rate * 3 / 4 ? 1 : 0) << ',' << envelope.doEnvelope() << '\n';
        }
        if (!result) throw std::runtime_error("Écriture CSV impossible");
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
