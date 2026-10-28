// GPL-3.0-or-later. Banc des effets Odin 2: vraies unités Source/audio/FX/*.cpp avec leur vrai JUCE.
//
// Usage: odin_fx_modules SCENARIOS STIMULI_DIR OUT_DIR RATE
// SCENARIOS: une ligne par scénario `nom genre stimulus clé=valeur ...`
//   (`clé@N=valeur` = changement appliqué avant la trame N). Écrit par scripts/compare_fx.py,
//   lu à l'identique par examples/compare_fx_render.rs: une seule source pour les réglages.
// STIMULI_DIR/<stimulus>.csv: frame,input,input_r. Sortie: OUT_DIR/<nom>.f64 =
//   (gauche, droite) en f64 little-endian entrelacés, une paire par trame.
#include "audio/FX/Chorus.h"
#include "audio/FX/Delay.h"
#include "audio/FX/Flanger.h"
#include <algorithm>
#include <cmath>
#include "audio/FX/Phaser.h"
#include "audio/FX/ZitaReverb.h"
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <map>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
struct Event {
    long frame;
    std::string key;
    double value;
};
struct Cfg {
    std::string name, kind, stimulus;
    std::map<std::string, double> base;
    std::vector<Event> events;
    float f(const std::string& key) const {
        const auto it = base.find(key);
        if (it == base.end()) throw std::runtime_error("clé absente: " + key + " (" + name + ")");
        return static_cast<float>(it->second);
    }
    bool b(const std::string& key) const { return f(key) > 0.5f; }
    int i(const std::string& key) const { return static_cast<int>(f(key)); }
};

float valueToDenominator(int value) {
    switch (value) {
    case 0: return 1.f;
    case 1: return 2.f;
    case 2: return 4.f;
    case 3: return 8.f;
    case 4: return 12.f;
    case 5: return 16.f;
    case 6: return 24.f;
    case 7: return 32.f;
    case 8: return 48.f;
    default: return 16.f;
    }
}

struct Runner {
    virtual ~Runner() = default;
    virtual void set(const std::string& key, float value) = 0;
    virtual void process(float& left, float& right) = 0;
};

struct DelayRunner : Runner {
    std::unique_ptr<Delay> d = std::make_unique<Delay>();
    float mod_time = 0, mod_fb = 0, mod_hp = 0, mod_dry = 0, mod_wet = 0;
    float bpm = 120;
    bool sync = false;
    explicit DelayRunner(const Cfg& c, int rate) {
        d->setSampleRate(static_cast<float>(rate));
        d->setTimeModPointer(&mod_time);
        d->setFeedbackModPointer(&mod_fb);
        d->setHPFreqModPointer(&mod_hp);
        d->setDryModPointer(&mod_dry);
        d->setWetModPointer(&mod_wet);
        mod_time = c.f("mod_time"); mod_fb = c.f("mod_fb"); mod_hp = c.f("mod_hp");
        mod_dry = c.f("mod_dry"); mod_wet = c.f("mod_wet");
        d->setPingPong(c.b("pp"));
        d->setFeedback(c.f("fb"));
        d->setHPFreq(c.f("hp"));
        d->setDucking(c.f("duck"));
        d->setDry(c.f("dry"));
        d->setWet(c.f("wet"));
        sync = c.b("sync");
        bpm = c.f("bpm");
        d->setSynctimeNumerator(c.f("num"));
        d->setSynctimeDenominator(c.f("den"));
        if (sync) d->setFreqBPM(bpm); else d->setDelayTime(c.f("time"));
        if (c.b("reset")) d->reset();
    }
    void set(const std::string& k, float v) override {
        if (k == "time") d->setDelayTime(v);
        else if (k == "fb") d->setFeedback(v);
        else if (k == "dry") d->setDry(v);
        else if (k == "wet") d->setWet(v);
        else if (k == "hp") d->setHPFreq(v);
        else if (k == "duck") d->setDucking(v);
        else if (k == "pp") d->setPingPong(v > 0.5f);
        else if (k == "bpm") { bpm = v; if (sync) d->setFreqBPM(bpm); }
        else if (k == "mod_time") mod_time = v;
        else if (k == "mod_fb") mod_fb = v;
        else if (k == "mod_hp") mod_hp = v;
        else if (k == "mod_dry") mod_dry = v;
        else if (k == "mod_wet") mod_wet = v;
        else if (k == "reset") d->reset();
        else throw std::runtime_error("delay: clé dynamique inconnue " + k);
    }
    void process(float& l, float& r) override {
        l = d->doDelayLeft(l);
        r = d->doDelayRight(r);
    }
};

struct ChorusRunner : Runner {
    Chorus ch[2];
    float mod_freq = 0, mod_amount = 0, mod_fb = 0, mod_drywet = 0;
    float bpm = 120;
    bool sync = false;
    explicit ChorusRunner(const Cfg& c, int rate) {
        ch[1].setLFOResetPos(0.25); // initializeModules
        for (auto& x : ch) {
            x.setSampleRate(static_cast<float>(rate));
            x.setFreqModPointer(&mod_freq);
            x.setAmountModPointer(&mod_amount);
            x.setFeedbackModPointer(&mod_fb);
            x.setDryWetModPointer(&mod_drywet);
        }
        mod_freq = c.f("mod_freq"); mod_amount = c.f("mod_amount");
        mod_fb = c.f("mod_fb"); mod_drywet = c.f("mod_drywet");
        sync = c.b("sync");
        bpm = c.f("bpm");
        for (auto& x : ch) {
            x.setAmount(c.f("amount"));
            x.setFeedback(c.f("fb"));
            x.setDryWet(c.f("drywet"));
            x.setSynctimeNumerator(c.f("num"));
            x.setSynctimeDenominator(c.f("den"));
            if (sync) x.setFreqBPM(bpm); else x.setLFOFreq(c.f("rate"));
            if (c.b("reset")) x.reset();
            if (c.b("lforeset")) x.resetLFO();
        }
    }
    void set(const std::string& k, float v) override {
        for (auto& x : ch) {
            if (k == "amount") x.setAmount(v);
            else if (k == "rate") x.setLFOFreq(v);
            else if (k == "fb") x.setFeedback(v);
            else if (k == "drywet") x.setDryWet(v);
            else if (k == "bpm") { bpm = v; if (sync) x.setFreqBPM(bpm); }
            else if (k == "lforeset") x.resetLFO();
            else if (k == "mod_freq") mod_freq = v;
            else if (k == "mod_amount") mod_amount = v;
            else if (k == "mod_fb") mod_fb = v;
            else if (k == "mod_drywet") mod_drywet = v;
            else throw std::runtime_error("chorus: clé dynamique inconnue " + k);
        }
    }
    void process(float& l, float& r) override {
        l = ch[0].doChorus(l);
        r = ch[1].doChorus(r);
    }
};

struct FlangerRunner : Runner {
    Flanger fl[2];
    float mod_freq = 0, mod_amount = 0, mod_fb = 0, mod_drywet = 0;
    float bpm = 120;
    bool sync = false;
    explicit FlangerRunner(const Cfg& c, int rate) {
        fl[1].setLFOResetPos(0.5); // initializeModules
        for (auto& x : fl) {
            x.setSampleRate(static_cast<float>(rate));
            x.setFreqModPointer(&mod_freq);
            x.setAmountModPointer(&mod_amount);
            x.setFeedbackModPointer(&mod_fb);
            x.setDryWetModPointer(&mod_drywet);
        }
        mod_freq = c.f("mod_freq"); mod_amount = c.f("mod_amount");
        mod_fb = c.f("mod_fb"); mod_drywet = c.f("mod_drywet");
        sync = c.b("sync");
        bpm = c.f("bpm");
        for (auto& x : fl) {
            x.setLFOAmount(c.f("amount"));
            x.setFeedback(c.f("fb"));
            x.setDryWet(c.f("drywet"));
            x.setSynctimeNumerator(c.f("num"));
            x.setSynctimeDenominator(c.f("den"));
            if (sync) x.setFreqBPM(bpm); else x.setLFOFreq(c.f("rate"));
            if (c.b("reset")) x.reset();
            if (c.b("lforeset")) x.resetLFO();
        }
    }
    void set(const std::string& k, float v) override {
        for (auto& x : fl) {
            if (k == "amount") x.setLFOAmount(v);
            else if (k == "rate") x.setLFOFreq(v);
            else if (k == "fb") x.setFeedback(v);
            else if (k == "drywet") x.setDryWet(v);
            else if (k == "bpm") { bpm = v; if (sync) x.setFreqBPM(bpm); }
            else if (k == "lforeset") x.resetLFO();
            else if (k == "mod_freq") mod_freq = v;
            else if (k == "mod_amount") mod_amount = v;
            else if (k == "mod_fb") mod_fb = v;
            else if (k == "mod_drywet") mod_drywet = v;
            else throw std::runtime_error("flanger: clé dynamique inconnue " + k);
        }
    }
    void process(float& l, float& r) override {
        l = fl[0].doFlanger(l);
        r = fl[1].doFlanger(r);
    }
};

struct PhaserRunner : Runner {
    Phaser p;
    float mod_rate = 0, mod_amount = 0, mod_freq = 0, mod_fb = 0, mod_drywet = 0;
    float bpm = 120;
    bool sync = false;
    explicit PhaserRunner(const Cfg& c, int rate) {
        p.setSampleRate(static_cast<float>(rate));
        p.setRadiusBase(c.f("radius"));
        p.setRateModPointer(&mod_rate);
        p.setAmountModPointer(&mod_amount);
        p.setFreqModPointer(&mod_freq);
        p.setFeedbackModPointer(&mod_fb);
        p.setDryWetModPointer(&mod_drywet);
        mod_rate = c.f("mod_rate"); mod_amount = c.f("mod_amount"); mod_freq = c.f("mod_freq");
        mod_fb = c.f("mod_fb"); mod_drywet = c.f("mod_drywet");
        sync = c.b("sync");
        bpm = c.f("bpm");
        p.setLFOAmplitude(c.f("amount"));
        p.setFeedback(c.f("fb"));
        p.setBaseFreq(c.f("freq"));
        p.setDryWet(c.f("drywet"));
        p.setSynctimeNumerator(c.f("num"));
        p.setSynctimeDenominator(c.f("den"));
        if (sync) p.setFreqBPM(bpm); else p.setLFOFreq(c.f("rate"));
        if (c.b("reset")) p.reset();
        if (c.b("lforeset")) p.resetLFO();
    }
    void set(const std::string& k, float v) override {
        if (k == "amount") p.setLFOAmplitude(v);
        else if (k == "rate") p.setLFOFreq(v);
        else if (k == "fb") p.setFeedback(v);
        else if (k == "freq") p.setBaseFreq(v);
        else if (k == "drywet") p.setDryWet(v);
        else if (k == "bpm") { bpm = v; if (sync) p.setFreqBPM(bpm); }
        else if (k == "lforeset") p.resetLFO();
        else if (k == "mod_rate") mod_rate = v;
        else if (k == "mod_amount") mod_amount = v;
        else if (k == "mod_freq") mod_freq = v;
        else if (k == "mod_fb") mod_fb = v;
        else if (k == "mod_drywet") mod_drywet = v;
        else throw std::runtime_error("phaser: clé dynamique inconnue " + k);
    }
    void process(float& l, float& r) override {
        l = p.doPhaserLeft(l);
        r = p.doPhaserRight(r);
    }
};

struct ReverbRunner : Runner {
    std::unique_ptr<ZitaReverb> z = std::make_unique<ZitaReverb>();
    explicit ReverbRunner(const Cfg& c, int rate) {
        z->setSampleRate(static_cast<float>(rate));
        z->prepare();
        z->set_delay(c.f("delay") / 1000.f);
        z->set_rtmid(c.f("mid"));
        z->set_fdamp(c.f("damp"));
        z->set_eq1_gain(c.f("eqgain"));
        z->set_eq1_freq(c.f("eqfreq"));
        z->set_opmix(c.f("drywet"));
        if (c.b("lowpar")) {
            z->set_xover(c.f("xover"));
            z->set_rtlow(c.f("rtlow"));
        }
        if (c.b("reset")) z->reset();
    }
    void set(const std::string& k, float v) override {
        if (k == "delay") z->set_delay(v / 1000.f);
        else if (k == "mid") z->set_rtmid(v);
        else if (k == "damp") z->set_fdamp(v);
        else if (k == "eqgain") z->set_eq1_gain(v);
        else if (k == "eqfreq") z->set_eq1_freq(v);
        else if (k == "drywet") z->set_opmix(v);
        else throw std::runtime_error("reverb: clé dynamique inconnue " + k);
    }
    void process(float& l, float& r) override {
        float io[2] = {l, r};
        z->process(io);
        l = io[0];
        r = io[1];
    }
};

// Réplique la section FX de PluginProcessorProcess.cpp:441-472 avec les réglages
// de PluginProcessorValueChange.cpp / PluginProcessorMisc.cpp (initializeModules, setSampleRate, setBPM).
struct ChainRunner : Runner {
    std::unique_ptr<Delay> delay = std::make_unique<Delay>();
    Phaser phaser;
    Flanger flanger[2];
    Chorus chorus[2];
    std::unique_ptr<ZitaReverb> reverb = std::make_unique<ZitaReverb>();
    float zero = 0.f;
    bool delay_on, phaser_on, flanger_on, chorus_on, reverb_on;
    bool delay_sync, phaser_sync, flanger_sync, chorus_sync;
    bool phaser_reset, flanger_reset, chorus_reset;
    int position[5];
    float bpm;
    float delay_time, chorus_rate, flanger_rate, phaser_rate;
    explicit ChainRunner(const Cfg& c, int rate) {
        phaser.setRadiusBase(1.25f);
        chorus[1].setLFOResetPos(0.25);
        flanger[1].setLFOResetPos(0.5);
        delay->setTimeModPointer(&zero); delay->setFeedbackModPointer(&zero);
        delay->setHPFreqModPointer(&zero); delay->setDryModPointer(&zero); delay->setWetModPointer(&zero);
        phaser.setRateModPointer(&zero); phaser.setAmountModPointer(&zero); phaser.setDryWetModPointer(&zero);
        phaser.setFreqModPointer(&zero); phaser.setFeedbackModPointer(&zero);
        for (int s = 0; s < 2; ++s) {
            flanger[s].setFreqModPointer(&zero); flanger[s].setAmountModPointer(&zero);
            flanger[s].setFeedbackModPointer(&zero); flanger[s].setDryWetModPointer(&zero);
            chorus[s].setFreqModPointer(&zero); chorus[s].setAmountModPointer(&zero);
            chorus[s].setFeedbackModPointer(&zero); chorus[s].setDryWetModPointer(&zero);
        }
        const float sr = static_cast<float>(rate);
        for (int s = 0; s < 2; ++s) { flanger[s].setSampleRate(sr); chorus[s].setSampleRate(sr); }
        delay->setSampleRate(sr);
        phaser.setSampleRate(sr);
        reverb->setSampleRate(sr);
        reverb->prepare();

        delay_on = c.b("delay_on"); phaser_on = c.b("phaser_on"); flanger_on = c.b("flanger_on");
        chorus_on = c.b("chorus_on"); reverb_on = c.b("reverb_on");
        position[0] = c.i("pos_delay"); position[1] = c.i("pos_phaser"); position[2] = c.i("pos_flanger");
        position[3] = c.i("pos_chorus"); position[4] = c.i("pos_reverb");
        bpm = c.f("bpm");
        phaser_reset = c.b("phaser_reset"); flanger_reset = c.b("flanger_reset"); chorus_reset = c.b("chorus_reset");

        delay_time = c.f("delay_time");
        delay->setDelayTime(delay_time);
        delay->setFeedback(c.f("delay_fb"));
        delay->setHPFreq(c.f("delay_hp"));
        delay->setDucking(c.f("delay_duck"));
        delay->setDry(c.f("delay_dry"));
        delay->setWet(c.f("delay_wet"));
        delay->setPingPong(c.f("delay_pp") > 0.5f);
        delay->setSynctimeNumerator(c.f("delay_num") + 1);
        delay->setSynctimeDenominator(valueToDenominator(c.i("delay_den")));
        delay_sync = c.b("delay_sync");

        phaser.setLFOAmplitude(c.f("phaser_amount"));
        phaser.setFeedback(c.f("phaser_fb"));
        phaser.setBaseFreq(c.f("phaser_freq"));
        phaser_rate = c.f("phaser_rate");
        phaser.setLFOFreq(phaser_rate);
        phaser.setDryWet(c.f("phaser_drywet"));
        phaser.setSynctimeNumerator(c.f("phaser_num") + 1);
        phaser.setSynctimeDenominator(valueToDenominator(c.i("phaser_den")));
        phaser_sync = c.b("phaser_sync");

        flanger_rate = c.f("flanger_rate");
        chorus_rate = c.f("chorus_rate");
        for (int s = 0; s < 2; ++s) {
            flanger[s].setLFOAmount(c.f("flanger_amount"));
            flanger[s].setLFOFreq(flanger_rate);
            flanger[s].setFeedback(c.f("flanger_fb"));
            flanger[s].setDryWet(c.f("flanger_drywet"));
            flanger[s].setSynctimeNumerator(c.f("flanger_num") + 1);
            flanger[s].setSynctimeDenominator(valueToDenominator(c.i("flanger_den")));
            chorus[s].setAmount(c.f("chorus_amount"));
            chorus[s].setLFOFreq(chorus_rate);
            chorus[s].setFeedback(c.f("chorus_fb"));
            chorus[s].setDryWet(c.f("chorus_drywet"));
            chorus[s].setSynctimeNumerator(c.f("chorus_num") + 1);
            chorus[s].setSynctimeDenominator(valueToDenominator(c.i("chorus_den")));
        }
        flanger_sync = c.b("flanger_sync");
        chorus_sync = c.b("chorus_sync");

        reverb->set_delay(c.f("reverb_delay") / 1000.f);
        reverb->set_rtmid(c.f("reverb_mid"));
        reverb->set_fdamp(c.f("reverb_damp"));
        reverb->set_eq1_gain(c.f("reverb_eqgain"));
        reverb->set_eq1_freq(c.f("reverb_eqfreq"));
        reverb->set_opmix(c.f("reverb_drywet"));

        if (delay_on) delay->reset();
        if (reverb_on) reverb->reset();
        applyBpm();
    }
    // OdinAudioProcessor::setBPM, appelé à chaque bloc.
    void applyBpm() {
        if (delay_sync) delay->setFreqBPM(bpm);
        if (phaser_sync) phaser.setFreqBPM(bpm);
        if (flanger_sync) { flanger[0].setFreqBPM(bpm); flanger[1].setFreqBPM(bpm); }
        if (chorus_sync) { chorus[0].setFreqBPM(bpm); chorus[1].setFreqBPM(bpm); }
    }
    void set(const std::string& k, float v) override {
        if (k == "bpm") { bpm = v; applyBpm(); }
        else if (k == "noteon") { // PluginProcessorMidi.cpp:84-94
            if (phaser_reset) phaser.resetLFO();
            if (flanger_reset) { flanger[0].resetLFO(); flanger[1].resetLFO(); }
            if (chorus_reset) { chorus[0].resetLFO(); chorus[1].resetLFO(); }
        } else throw std::runtime_error("chain: clé dynamique inconnue " + k);
    }
    void process(float& l, float& r) override {
        float sig[2] = {l, r};
        for (int slot = 0; slot < 5; ++slot) {
            if (position[0] == slot) {
                if (delay_on) { sig[0] = delay->doDelayLeft(sig[0]); sig[1] = delay->doDelayRight(sig[1]); }
            } else if (position[1] == slot) {
                if (phaser_on) { sig[0] = phaser.doPhaserLeft(sig[0]); sig[1] = phaser.doPhaserRight(sig[1]); }
            } else if (position[2] == slot) {
                if (flanger_on) { sig[0] = flanger[0].doFlanger(sig[0]); sig[1] = flanger[1].doFlanger(sig[1]); }
            } else if (position[3] == slot) {
                if (chorus_on) { sig[0] = chorus[0].doChorus(sig[0]); sig[1] = chorus[1].doChorus(sig[1]); }
            } else if (position[4] == slot) {
                if (reverb_on) reverb->process(sig);
            }
        }
        l = sig[0];
        r = sig[1];
    }
};

std::unique_ptr<Runner> make_runner(const Cfg& c, int rate) {
    if (c.kind == "delay") return std::make_unique<DelayRunner>(c, rate);
    if (c.kind == "chorus") return std::make_unique<ChorusRunner>(c, rate);
    if (c.kind == "flanger") return std::make_unique<FlangerRunner>(c, rate);
    if (c.kind == "phaser") return std::make_unique<PhaserRunner>(c, rate);
    if (c.kind == "reverb") return std::make_unique<ReverbRunner>(c, rate);
    if (c.kind == "chain") return std::make_unique<ChainRunner>(c, rate);
    throw std::runtime_error("genre inconnu: " + c.kind);
}

std::vector<Cfg> read_scenarios(const std::filesystem::path& path) {
    std::ifstream in(path);
    if (!in) throw std::runtime_error("scénarios illisibles: " + path.string());
    std::vector<Cfg> result;
    std::string line;
    while (std::getline(in, line)) {
        if (line.empty() || line[0] == '#') continue;
        std::istringstream words(line);
        Cfg c;
        words >> c.name >> c.kind >> c.stimulus;
        std::string item;
        while (words >> item) {
            const auto eq = item.find('=');
            if (eq == std::string::npos) throw std::runtime_error("réglage invalide: " + item);
            std::string key = item.substr(0, eq);
            const double value = std::stod(item.substr(eq + 1));
            const auto at = key.find('@');
            if (at == std::string::npos) c.base[key] = value;
            else c.events.push_back({std::stol(key.substr(at + 1)), key.substr(0, at), value});
        }
        result.push_back(std::move(c));
    }
    return result;
}

struct Stimulus {
    std::vector<float> left, right;
};
Stimulus read_stimulus(const std::filesystem::path& path) {
    std::ifstream in(path);
    if (!in) throw std::runtime_error("stimulus illisible: " + path.string());
    Stimulus s;
    std::string line;
    std::getline(in, line);
    while (std::getline(in, line)) {
        const auto a = line.find(',');
        const auto b = line.find(',', a + 1);
        if (a == std::string::npos || b == std::string::npos) throw std::runtime_error("CSV stimulus incorrect");
        s.left.push_back(std::stof(line.substr(a + 1, b - a - 1)));
        s.right.push_back(std::stof(line.substr(b + 1)));
    }
    return s;
}
} // namespace

int main(int argc, char** argv) {
    try {
        if (argc != 5) throw std::runtime_error("Usage: odin_fx_modules SCENARIOS STIMULI_DIR OUT_DIR RATE");
        const std::filesystem::path stimuli(argv[2]), output(argv[3]);
        const int rate = std::stoi(argv[4]);
        std::filesystem::create_directories(output);
        std::map<std::string, Stimulus> cache;
        for (auto& scenario : read_scenarios(argv[1])) {
            auto it = cache.find(scenario.stimulus);
            if (it == cache.end()) it = cache.emplace(scenario.stimulus, read_stimulus(stimuli / (scenario.stimulus + ".csv"))).first;
            const Stimulus& in = it->second;
            auto runner = make_runner(scenario, rate);
            auto events = scenario.events;
            std::stable_sort(events.begin(), events.end(), [](const Event& a, const Event& b) { return a.frame < b.frame; });
            size_t next = 0;
            std::vector<double> out;
            out.reserve(in.left.size() * 2);
            for (size_t frame = 0; frame < in.left.size(); ++frame) {
                while (next < events.size() && events[next].frame == static_cast<long>(frame)) {
                    runner->set(events[next].key, static_cast<float>(events[next].value));
                    ++next;
                }
                float l = in.left[frame], r = in.right[frame];
                runner->process(l, r);
                if (!std::isfinite(l) || !std::isfinite(r)) throw std::runtime_error("sortie non finie: " + scenario.name);
                out.push_back(l);
                out.push_back(r);
            }
            std::ofstream file(output / (scenario.name + ".f64"), std::ios::binary);
            file.write(reinterpret_cast<const char*>(out.data()), static_cast<std::streamsize>(out.size() * sizeof(double)));
            if (!file) throw std::runtime_error("écriture impossible: " + scenario.name);
        }
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
