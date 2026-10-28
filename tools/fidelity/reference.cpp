// GPL-3.0-or-later. Aucun éditeur ni périphérique audio n'est créé.
#include "PluginProcessor.h"
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
struct Event { double seconds; bool on; int note; };
class Clock final : public juce::AudioPlayHead {
public:
    double seconds = 0.0;
    bool getCurrentPosition(CurrentPositionInfo& info) override {
        info.resetToDefault();
        info.bpm = 120.0;
        info.timeInSeconds = seconds;
        info.ppqPosition = seconds * 2.0;
        info.isPlaying = true;
        return true;
    }
};
void word(std::ostream& stream, uint32_t value, int bytes) {
    for (int i = 0; i < bytes; ++i) stream.put(static_cast<char>((value >> (8 * i)) & 255));
}
void header(std::ostream& stream, uint32_t rate, uint32_t frames) {
    const uint32_t size = frames * 8;
    stream.write("RIFF", 4); word(stream, 36 + size, 4); stream.write("WAVEfmt ", 8);
    word(stream, 16, 4); word(stream, 3, 2); word(stream, 2, 2); word(stream, rate, 4);
    word(stream, rate * 8, 4); word(stream, 8, 2); word(stream, 32, 2);
    stream.write("data", 4); word(stream, size, 4);
}
struct Override { std::string name; std::string text; };
double parse_number(const std::string& text, const std::string& name) {
    size_t used = 0;
    double value = 0.0;
    try { value = std::stod(text, &used); } catch (const std::exception&) { used = 0; }
    if (used == 0 || used != text.size() || !std::isfinite(value))
        throw std::runtime_error("--set " + name + ": valeur numérique invalide '" + text + "'");
    return value;
}
// "nom=valeur" -> paramètre audio (noeud PARAM, propriété id == nom, propriété value physique);
// "noeud/propriété=valeur" -> propriété d'un noeud non audio (misc, osc, fx, lfo, mod, draw).
// Le type de la propriété existante (bool, int, double) est conservé.
void apply_override(juce::ValueTree& tree, juce::AudioProcessor& processor, const Override& item) {
    const auto slash = item.name.find('/');
    if (slash == std::string::npos) {
        const double value = parse_number(item.text, item.name);
        for (auto child : tree) {
            if (child.getType().toString() == "PARAM" && child["id"].toString().toStdString() == item.name) {
                child.setProperty("value", value, nullptr);
                return;
            }
        }
        // Paramètre absent du patch (valeur par défaut à la lecture): le créer s'il existe dans le processeur.
        for (auto* parameter : processor.getParameters()) {
            auto* identified = dynamic_cast<juce::AudioProcessorParameterWithID*>(parameter);
            if (identified != nullptr && identified->paramID.toStdString() == item.name) {
                juce::ValueTree created("PARAM");
                created.setProperty("id", juce::String(item.name), nullptr);
                created.setProperty("value", value, nullptr);
                tree.appendChild(created, nullptr);
                return;
            }
        }
        throw std::runtime_error("--set " + item.name + ": paramètre audio inconnu");
    }
    const std::string node_name = item.name.substr(0, slash);
    const std::string property = item.name.substr(slash + 1);
    if (node_name == "PARAM") throw std::runtime_error("--set " + item.name + ": utiliser nom=valeur pour un paramètre audio");
    auto node = tree.getChildWithName(juce::Identifier(node_name));
    if (!node.isValid()) throw std::runtime_error("--set " + item.name + ": noeud '" + node_name + "' absent");
    const juce::Identifier key(property);
    if (!node.hasProperty(key)) throw std::runtime_error("--set " + item.name + ": propriété absente du noeud '" + node_name + "'");
    const juce::var current = node[key];
    if (current.isString()) { node.setProperty(key, juce::String::fromUTF8(item.text.c_str()), nullptr); return; }
    const double value = parse_number(item.text, item.name);
    if (current.isBool()) node.setProperty(key, value != 0.0, nullptr);
    else if (current.isInt() || current.isInt64()) {
        if (value != std::floor(value) || std::fabs(value) > 2147483647.0)
            throw std::runtime_error("--set " + item.name + ": entier attendu, reçu '" + item.text + "'");
        node.setProperty(key, static_cast<int>(value), nullptr);
    } else if (current.isDouble()) node.setProperty(key, value, nullptr);
    else throw std::runtime_error("--set " + item.name + ": type de propriété non modifiable");
}
}
int main(int argc, char** argv) {
    try {
        const char* usage = "Usage: odin_reference PATCH OUT.wav RATE note|pattern SECONDS [--set nom=valeur | --set noeud/propriété=valeur]...";
        if (argc < 6) throw std::runtime_error(usage);
        std::vector<Override> overrides;
        for (int i = 6; i < argc; ++i) {
            if (std::string(argv[i]) != "--set" || i + 1 >= argc) throw std::runtime_error(usage);
            const std::string assignment(argv[++i]);
            const auto equals = assignment.find('=');
            if (equals == std::string::npos || equals == 0) throw std::runtime_error("--set attend nom=valeur: " + assignment);
            overrides.push_back({assignment.substr(0, equals), assignment.substr(equals + 1)});
        }
        const int rate = std::stoi(argv[3]);
        const double seconds = std::stod(argv[5]);
        if (rate < 8000 || !std::isfinite(seconds) || seconds < 3 || seconds * rate > 500000000)
            throw std::runtime_error("Durée/fréquence hors limites WAV");
        const std::string scenario(argv[4]);
        const std::vector<Event> events = scenario == "note"
            ? std::vector<Event>{{0.25, true, 48}, {2.25, false, 48}}
            : std::vector<Event>{{0.25, true, 48}, {0.75, true, 55}, {1.25, false, 48},
                                 {1.25, true, 60}, {1.75, false, 55}, {2.25, false, 60}};
        if (scenario != "note" && scenario != "pattern") throw std::runtime_error("Scénario inconnu");
        const juce::ScopedJuceInitialiser_GUI juce;
        auto processor = std::make_unique<OdinAudioProcessor>();
        // Voice::Voice appelle srand(time): réinitialiser après les 24 constructions.
        std::srand(0);
        Clock clock;
        processor->setPlayHead(&clock);
        processor->setRateAndBufferSizeDetails(rate, 256);
        processor->prepareToPlay(rate, 256);
        juce::FileInputStream input(juce::File(juce::String::fromUTF8(argv[1])));
        if (!input.openedOk()) throw std::runtime_error("Preset introuvable");
        auto tree = juce::ValueTree::readFromStream(input);
        if (!tree.isValid() || tree.getType().toString() != "Odin")
            throw std::runtime_error("ValueTree Odin invalide");
        for (const auto& item : overrides) apply_override(tree, *processor, item);
        // Le chemin état reconstruit aussi les trois tables draw, contrairement à readPatch seul.
        const auto xml = tree.createXml();
        juce::MemoryBlock state;
        juce::AudioProcessor::copyXmlToBinary(*xml, state);
        processor->setStateInformation(state.getData(), static_cast<int>(state.getSize()));
        // Le chargement d'état ne doit pas laisser une graine dépendant du temps.
        // Limites: initialisations aléatoires du processeur avant cette ligne et
        // std::random_device de l'arpégiateur restent ceux de l'amont.
        std::srand(0);
        std::ofstream output(argv[2], std::ios::binary);
        if (!output) throw std::runtime_error("Ouverture WAV impossible");
        const auto frames = static_cast<uint32_t>(std::llround(seconds * rate));
        header(output, rate, frames);
        size_t event = 0;
        for (uint32_t cursor = 0; cursor < frames;) {
            const auto count = static_cast<int>(std::min(uint32_t(256), frames - cursor));
            juce::AudioBuffer<float> buffer(2, count);
            buffer.clear();
            juce::MidiBuffer midi;
            while (event < events.size()) {
                const auto at = static_cast<uint32_t>(std::llround(events[event].seconds * rate));
                if (at >= cursor + count) break;
                const auto& item = events[event++];
                midi.addEvent(item.on ? juce::MidiMessage::noteOn(1, item.note, uint8_t(100))
                                      : juce::MidiMessage::noteOff(1, item.note), static_cast<int>(at - cursor));
            }
            clock.seconds = double(cursor) / rate;
            processor->processBlock(buffer, midi);
            for (int frame = 0; frame < count; ++frame) {
                for (int channel = 0; channel < 2; ++channel) {
                    const float sample = buffer.getSample(channel, frame);
                    if (!std::isfinite(sample)) throw std::runtime_error("Échantillon C++ non fini");
                    uint32_t bits;
                    std::memcpy(&bits, &sample, sizeof bits);
                    word(output, bits, 4);
                }
            }
            cursor += count;
        }
        processor->setPlayHead(nullptr);
        processor->releaseResources();
        if (!output) throw std::runtime_error("Écriture WAV impossible");
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
