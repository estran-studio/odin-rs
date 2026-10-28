#!/usr/bin/env python3
"""Compare les oscillateurs et LFO Rust aux vraies unités Odin 2 (Source/audio/Oscillators, JUCE).

Couvre WavetableOsc2D, MultiOscillator, VectorOscillator, ChiptuneOscillator, FM, PM, les
oscillateurs dessinés (wave/chip/spec), AnalogOscillator (pulsewidth, sync) et le LFO (23 formes,
sync tempo), plus les données (tables, mappings, tables 2D, tables dessinées).

  python3 scripts/compare_osc.py [--out DIR] [--build DIR] [--rate 44100] [--skip-cpp] [--only SUBSTR]

Le C++ (cible odin_osc_modules de tools/fidelity/CMakeLists.txt) est construit dans --build
(défaut target/osc-compare/build, configuré une fois, voir en-tête de main()).
"""
import argparse
import csv
import math
import os
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from fidelity import f32, stimuli as write_stimuli  # noqa: E402

THRESHOLD = 1e-3  # erreur absolue max, signaux d'amplitude ~0.25-1
STAT_TOLERANCE = 0.10  # écart relatif RMS / centroïde pour les scénarios aléatoires
JUCE = "/private/tmp/odin-fidelity-juce-2f980209"
TUNING = "/private/tmp/odin-fidelity-tuning-8eb7232a"
JSON = "/private/tmp/odin-fidelity-json-960b763e"


def write_freq(path, rate, frames, fn):
    with path.open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("frame", "input"))
        for frame in range(frames):
            writer.writerow((frame, format(f32(fn(frame / rate)), ".9g")))


def make_stimuli(directory, rate):
    write_stimuli(directory, rate)  # impulse/sine/sweep + frequency_tone (220 Hz) / frequency_sweep
    for name, hz in (("low", 100.0), ("mid", 440.0), ("high", 2093.0), ("vhigh", 6000.0), ("c2", 65.406)):
        write_freq(directory / f"osc_{name}.csv", rate, rate, lambda t, hz=hz: hz)
    # 110 Hz puis 330 Hz à 0,4 s: changement de note (glide)
    write_freq(directory / "osc_step.csv", rate, rate, lambda t: 110.0 if t < 0.4 else 330.0)
    # 2 s pour les spectres moyens
    write_freq(directory / "osc_long.csv", rate, rate * 2, lambda t: 220.0)


def build_cases():
    """(nom, type, stimulus, {clés})"""
    cases = []

    def add(name, type_, stim, **kv):
        cases.append((name, type_, stim, kv))

    def tag(value):
        return format(value, "g").replace("-", "m").replace(".", "p")

    # ------------------------------------------------------------ analogique (drift = 0)
    for wave, wname in enumerate(("saw", "pulse", "tri", "sine")):
        for stim in ("tone", "low", "high", "sweep"):
            file = {"tone": "frequency_tone", "sweep": "frequency_sweep"}.get(stim, f"osc_{stim}")
            add(f"analog_{wname}_{stim}", "analog", file, wave=wave, pw=0.5)
    for pw in (0.02, 0.1, 0.25, 0.75, 0.9, 0.98, 0.01, 0.995):
        add(f"analog_pulse_pw{tag(pw)}", "analog", "frequency_tone", wave=1, pw=pw)
    add("analog_pulse_pwm_pos", "analog", "frequency_tone", wave=1, pw=0.5, pwm=0.4)
    add("analog_pulse_pwm_neg", "analog", "frequency_tone", wave=1, pw=0.3, pwm=-0.8)
    add("analog_pulse_pwm_over", "analog", "frequency_tone", wave=1, pw=0.9, pwm=0.9)
    for ratio_name, semi in (("p7", 7), ("p12", 12), ("p5", 5), ("m5", -5)):
        for wave, wname in ((0, "saw"), (1, "pulse"), (3, "sine")):
            add(f"analog_sync_{wname}_{ratio_name}", "analog", "frequency_tone", wave=wave, pw=0.4, sync=1, semi=semi)
    add("analog_sync_sweep", "analog", "frequency_sweep", wave=0, sync=1, semi=7)
    add("analog_sync_high", "analog", "osc_high", wave=0, sync=1, semi=3)
    for oct_, semi, fine in ((-2, 5, 33), (1, -7, -45), (2, 0, 0), (-3, 11, 99), (3, 12, 0), (0, 0, 100), (-4, 0, 0)):
        add(f"analog_tune_o{tag(oct_)}_s{tag(semi)}_f{tag(fine)}", "analog", "frequency_tone", wave=0, oct=oct_, semi=semi, fine=fine)
    add("analog_exp_mod", "analog", "frequency_tone", wave=0, exp=0.1)
    add("analog_exp_neg", "analog", "frequency_tone", wave=3, exp=-0.15)
    add("analog_lin_mod", "analog", "frequency_tone", wave=3, lin=0.05)
    add("analog_lin_neg", "analog", "frequency_tone", wave=3, lin=-0.1)

    # ------------------------------------------------------------ wavetable 2D
    for wt in (1, 2, 3, 5, 9, 14, 18, 24, 29, 35):
        for pos in (0.0, 0.2, 0.5, 0.77, 1.0):
            add(f"wt2d_wt{wt}_pos{tag(pos)}_tone", "wt2d", "frequency_tone", wt=wt, pos=pos)
    for wt in (1, 6, 12, 19, 33):
        for stim in ("low", "high", "vhigh", "sweep"):
            file = "frequency_sweep" if stim == "sweep" else f"osc_{stim}"
            add(f"wt2d_wt{wt}_{stim}", "wt2d", file, wt=wt, pos=0.45)
    for pmod in (0.2, -0.3, 0.9):
        add(f"wt2d_pmod{tag(pmod)}", "wt2d", "frequency_tone", wt=7, pos=0.3, pmod=pmod)
    add("wt2d_posmod_env", "wt2d", "frequency_tone", wt=7, pos=0.3, pamt=0.8, psrc=0.6)
    add("wt2d_posmod_lfo_neg", "wt2d", "frequency_tone", wt=7, pos=0.6, pamt=-0.5, psrc=0.9)
    for oct_, semi, fine in ((-2, 5, 33), (2, 0, 0), (0, 7, -50)):
        add(f"wt2d_tune_o{tag(oct_)}_s{tag(semi)}_f{tag(fine)}", "wt2d", "frequency_tone", wt=10, pos=0.5, oct=oct_, semi=semi, fine=fine)
    add("wt2d_sync", "wt2d", "frequency_tone", wt=4, pos=0.5, sync=1, semi=7)
    add("wt2d_sync_sweep", "wt2d", "frequency_sweep", wt=4, pos=0.5, sync=1, semi=5)
    add("wt2d_exp", "wt2d", "frequency_tone", wt=8, pos=0.5, exp=0.07)
    add("wt2d_lin", "wt2d", "frequency_tone", wt=8, pos=0.5, lin=0.04)

    # ------------------------------------------------------------ multi
    for wt in (1, 7, 20, 31):
        for pos in (0.0, 0.4, 1.0):
            for detune in (0.0, 0.2, 0.6, 1.0):
                add(f"multi_wt{wt}_pos{tag(pos)}_d{tag(detune)}", "multi", "frequency_tone", wt=wt, pos=pos, detune=detune, spread=0.0)
    for spread in (0.25, 0.5, 1.0):
        for wt in (3, 18):
            add(f"multi_wt{wt}_spread{tag(spread)}", "multi", "frequency_tone", wt=wt, pos=0.5, detune=0.3, spread=spread)
    for stim in ("low", "high", "vhigh", "sweep"):
        file = "frequency_sweep" if stim == "sweep" else f"osc_{stim}"
        add(f"multi_{stim}", "multi", file, wt=7, pos=0.5, detune=0.5, spread=0.3)
    add("multi_pmod", "multi", "frequency_tone", wt=7, pos=0.2, detune=0.4, spread=0.2, pmod=0.35)
    add("multi_dmod", "multi", "frequency_tone", wt=7, pos=0.2, detune=0.2, spread=0.2, dmod=0.3)
    add("multi_dmod_neg", "multi", "frequency_tone", wt=7, pos=0.2, detune=0.2, spread=0.2, dmod=-0.9)
    add("multi_smod", "multi", "frequency_tone", wt=7, pos=0.2, detune=0.2, spread=0.2, smod=0.4)
    add("multi_tune", "multi", "frequency_tone", wt=9, pos=0.5, detune=0.5, spread=0.5, oct=-1, semi=4, fine=27)
    add("multi_exp", "multi", "frequency_tone", wt=9, pos=0.5, detune=0.5, spread=0.5, exp=0.05)
    add("multi_lin", "multi", "frequency_tone", wt=9, pos=0.5, detune=0.5, spread=0.5, lin=0.03)
    for seed in (1, 2, 3):  # phases aléatoires: spectre moyen seulement
        add(f"stat_multi_randphase{seed}", "multi", "osc_long", wt=7, pos=0.5, detune=0.4, spread=0.3, randphase=1, seed=seed)

    # ------------------------------------------------------------ vector
    corners = [(101, 102, 103, 104), (121, 122, 123, 124), (131, 132, 133, 134), (601, 602, 603, 101),
               (501, 502, 503, 504), (701, 702, 703, 101), (221, 222, 223, 224), (451, 452, 453, 454)]
    for index, (a, b, c, d) in enumerate(corners):
        for x, y in ((0.0, 0.0), (1.0, 1.0), (0.3, 0.7), (1.0, 0.0), (0.0, 1.0)):
            add(f"vector_set{index}_x{tag(x)}_y{tag(y)}", "vector", "frequency_tone", a=a, b=b, c=c, d=d, x=x, y=y)
    for stim in ("low", "high", "vhigh", "sweep"):
        file = "frequency_sweep" if stim == "sweep" else f"osc_{stim}"
        add(f"vector_{stim}", "vector", file, a=141, b=151, c=161, d=171, x=0.4, y=0.6)
    add("vector_xmod", "vector", "frequency_tone", x=0.3, y=0.3, xmod=0.4)
    add("vector_ymod_neg", "vector", "frequency_tone", x=0.3, y=0.5, ymod=-0.8)
    add("vector_sync", "vector", "frequency_tone", x=0.5, y=0.5, sync=1, semi=7)
    add("vector_tune", "vector", "frequency_tone", x=0.5, y=0.5, oct=-1, semi=3, fine=-21)

    # ------------------------------------------------------------ chiptune
    for wave in (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 701, 702, 703):
        add(f"chip_wave{wave}", "chiptune", "frequency_tone", wave=wave)
    for aspeed in (7.5, 20.0, 40.0):
        for s3on in (0, 1):
            add(f"chip_arp_{tag(aspeed)}_s3{s3on}", "chiptune", "frequency_tone", wave=2, arp=1, aspeed=aspeed, s3on=s3on)
    add("chip_arp_steps", "chiptune", "frequency_tone", wave=4, arp=1, aspeed=11.0, s1=-5, s2=3, s3=-12, s3on=1)
    add("chip_arp_amod", "chiptune", "frequency_tone", wave=2, arp=1, aspeed=10.0, s3on=1, amod=0.1)
    add("chip_arp_amod_neg", "chiptune", "frequency_tone", wave=2, arp=1, aspeed=30.0, s3on=1, amod=-0.1)
    add("chip_arp_off_s", "chiptune", "frequency_tone", wave=2, arp=0, aspeed=10.0)
    add("chip_arp_sweep", "chiptune", "frequency_sweep", wave=3, arp=1, aspeed=15.0)
    add("chip_arp_high", "chiptune", "osc_high", wave=3, arp=1, aspeed=15.0, s3on=1)
    add("chip_sync", "chiptune", "frequency_tone", wave=2, sync=1, semi=7)
    add("chip_tune", "chiptune", "frequency_tone", wave=5, oct=-1, semi=2, fine=40)
    for noise_hz, stim in (("c2", "osc_c2"), ("tone", "osc_long"), ("high", "osc_high")):
        add(f"stat_chip_noise_{noise_hz}", "chiptune", stim, noise=1, wave=1)

    # ------------------------------------------------------------ FM / PM
    waves = (1, 101, 121, 131, 201)
    for kind in ("fm", "pm"):
        for cwave in (1, 101, 131):
            for mwave in (1, 102):
                for amount in (0.05, 0.3, 1.0):
                    add(f"{kind}_c{cwave}_m{mwave}_a{tag(amount)}", kind, "frequency_tone", cwave=cwave, mwave=mwave, amount=amount)
        for cr, mr in ((1, 2), (3, 2), (7, 5), (2, 1), (1, 8)):
            add(f"{kind}_ratio{cr}_{mr}", kind, "frequency_tone", cwave=1, mwave=1, amount=0.4, cr=cr, mr=mr)
        for stim in ("low", "high", "vhigh", "sweep"):
            file = "frequency_sweep" if stim == "sweep" else f"osc_{stim}"
            add(f"{kind}_{stim}", kind, file, cwave=101, mwave=1, amount=0.3, cr=1, mr=2)
        add(f"{kind}_amod", kind, "frequency_tone", cwave=1, mwave=1, amount=0.2, amod=0.3)
        add(f"{kind}_amod_neg", kind, "frequency_tone", cwave=1, mwave=1, amount=0.2, amod=-0.5)
        add(f"{kind}_crmod", kind, "frequency_tone", cwave=1, mwave=1, amount=0.3, crmod=0.05, cr=1, mr=2)
        add(f"{kind}_mrmod", kind, "frequency_tone", cwave=1, mwave=1, amount=0.3, mrmod=0.05, cr=1, mr=2)
        add(f"{kind}_tune", kind, "frequency_tone", cwave=101, mwave=1, amount=0.3, oct=-1, semi=3, fine=-30, cr=1, mr=2)
        add(f"{kind}_exp", kind, "frequency_tone", cwave=1, mwave=1, amount=0.2, exp=0.05)
        add(f"{kind}_lin", kind, "frequency_tone", cwave=1, mwave=1, amount=0.2, lin=0.03)
    for cwave, mwave in ((131, 121), (141, 1), (601, 1), (1, 701)):
        add(f"fm_extra_{cwave}_{mwave}", "fm", "frequency_tone", cwave=cwave, mwave=mwave, amount=0.3, cr=1, mr=2)
        add(f"pm_extra_{cwave}_{mwave}", "pm", "frequency_tone", cwave=cwave, mwave=mwave, amount=0.3, cr=1, mr=2)
    for amount in (0.05, 0.3, 1.0):
        for stim in ("tone", "low", "high"):
            file = "frequency_tone" if stim == "tone" else f"osc_{stim}"
            add(f"fm_exp_a{tag(amount)}_{stim}", "fm", file, cwave=1, mwave=1, amount=amount, expfm=1, cr=1, mr=2)
    add("fm_exp_amod", "fm", "frequency_tone", cwave=1, mwave=1, amount=0.1, expfm=1, amod=0.2, cr=1, mr=2)
    add("fm_exp_mrmod", "fm", "frequency_tone", cwave=1, mwave=1, amount=0.1, expfm=1, mrmod=0.03, cr=1, mr=2)

    # ------------------------------------------------------------ bruit (spectre moyen)
    for lp, hp in ((18000, 80), (1000, 80), (20000, 20), (5000, 2000), (400, 100), (12000, 8000)):
        add(f"stat_noise_lp{lp}_hp{hp}", "noise", "osc_long", lp=lp, hp=hp)
    add("stat_noise_lmod", "noise", "osc_long", lp=3000, hp=80, lmod=-0.1)
    add("stat_noise_hmod", "noise", "osc_long", lp=18000, hp=300, hmod=0.05)

    # ------------------------------------------------------------ oscillateurs dessinés
    for kind, kname in enumerate(("wave", "chip", "spec")):
        for idx in range(3):
            for stim in ("tone", "low", "high", "sweep"):
                file = {"tone": "frequency_tone", "sweep": "frequency_sweep"}.get(stim, f"osc_{stim}")
                add(f"draw_{kname}{idx}_{stim}", "wt1d", file, kind=kind, idx=idx)

    # ------------------------------------------------------------ voix: glide, détune d'unisson, pitch bend
    for g in (0.3, 0.6, 0.9, 1.0):
        add(f"voice_glide_up_g{tag(g)}", "voice", "osc_long", note=69, last=57, glide=g)
        add(f"voice_glide_down_g{tag(g)}", "voice", "osc_long", note=45, last=60, glide=g)
    add("voice_glide_threshold", "voice", "osc_long", note=69, last=57, glide=0.011)
    add("voice_glide_off", "voice", "osc_long", note=69, last=57, glide=0.0)
    add("voice_glide_saw", "voice", "osc_long", note=69, last=57, glide=0.5, wave=0)
    for unison in (0.37, -0.52, 0.08, 1.9):
        add(f"voice_unison_{tag(unison)}", "voice", "osc_long", note=60, unison=unison)
    for bend in (6.0, -3.0, 1.5, 12.0, -12.0):
        add(f"voice_bend_{tag(bend)}", "voice", "osc_long", note=64, bend=bend)
    add("voice_glide_unison_bend", "voice", "osc_long", note=72, last=55, glide=0.7, unison=0.21, bend=3.0)
    for note in (21, 33, 48, 69, 84, 96, 108):
        add(f"voice_note{note}", "voice", "osc_long", note=note)

    # ------------------------------------------------------------ LFO
    for wave in range(23):
        if wave == 7:
            continue
        add(f"lfo_wave{wave}_7p3", "lfo", "frequency_tone", wave=wave, freq=7.3)
        add(f"lfo_wave{wave}_150", "lfo", "frequency_tone", wave=wave, freq=150.0)
    for freq in (0.5, 20.0, 1000.0):
        add(f"lfo_sine_f{tag(freq)}", "lfo", "frequency_tone", wave=0, freq=freq)
        add(f"lfo_tri_f{tag(freq)}", "lfo", "frequency_tone", wave=2, freq=freq)
    for mod in (0.25, -0.4, 0.9):
        add(f"lfo_mod{tag(mod)}", "lfo", "frequency_tone", wave=2, freq=5.0, mod=mod)
    for bpm in (120.0, 137.5, 60.0):
        for num in (0, 2, 5):
            for den in range(9):
                add(f"lfo_sync_b{tag(bpm)}_n{num}_d{den}", "lfo", "frequency_tone", wave=2, sync=1, bpm=bpm, num=num, den=den)
    add("lfo_sync_mod", "lfo", "frequency_tone", wave=0, sync=1, bpm=140.0, num=0, den=2, mod=0.2)
    add("lfo_sync_badden", "lfo", "frequency_tone", wave=0, sync=1, bpm=140.0, num=1, den=11)
    add("stat_lfo_sh_20", "lfo", "osc_long", wave=7, freq=20.0)
    add("stat_lfo_sh_200", "lfo", "osc_long", wave=7, freq=200.0)
    return cases


def write_spec(path, cases, draw_seed):
    with path.open("w") as stream:
        stream.write(f"# drawseed {draw_seed}\n")
        for name, kind, stim, kv in cases:
            stream.write(f"{name} {kind} {stim}.csv " + " ".join(f"{k}={v}" for k, v in kv.items()) + "\n")


def run(command, log):
    with log.open("w") as stream:
        result = subprocess.run([str(c) for c in command], stdout=stream, stderr=subprocess.STDOUT, cwd=ROOT)
    if result.returncode != 0:
        print(f"ÉCHEC: {' '.join(str(c) for c in command)} (voir {log})")
        print(log.read_text()[-2000:])
        sys.exit(1)


def read_output(path):
    return np.loadtxt(path, delimiter=",", skiprows=1, usecols=(2,))


def centroid(signal, rate):
    spectrum = np.abs(np.fft.rfft(signal * np.hanning(len(signal))))
    freqs = np.fft.rfftfreq(len(signal), 1.0 / rate)
    total = spectrum.sum()
    return float((freqs * spectrum).sum() / total) if total > 0 else 0.0


def crossings(signal):
    index = np.where((signal[:-1] < 0.0) & (signal[1:] >= 0.0))[0]
    return index + (-signal[index]) / (signal[index + 1] - signal[index])


def compare_streams(out, cases, rate):
    rows = []
    for name, kind, _stim, _kv in cases:
        cpp, rust = read_output(out / "cpp" / f"{name}.csv"), read_output(out / "rust" / f"{name}.csv")
        if len(cpp) != len(rust):
            rows.append((name, kind, "LEN", len(cpp), len(rust), "FAIL"))
            continue
        if name.startswith("stat_lfo_sh"):
            # Échantillonneur-bloqueur: nombre de changements de valeur (frontières de cycle) et, avec
            # assez de tirages, RMS d'une loi uniforme sur [-1,1] (0,577); les valeurs ne sont pas reproductibles.
            changes_c, changes_r = int(np.count_nonzero(np.diff(cpp))), int(np.count_nonzero(np.diff(rust)))
            rms_c, rms_r = float(np.sqrt(np.mean(cpp**2))), float(np.sqrt(np.mean(rust**2)))
            status = "OK" if abs(changes_c - changes_r) <= 1 and (changes_c < 100 or abs(rms_c - rms_r) < 0.06) else "FAIL"
            rows.append((name, kind, f"changements {changes_c}/{changes_r}", f"rms {rms_c:.3f}/{rms_r:.3f}", "", status))
            continue
        if name.startswith("stat_"):
            rms_c, rms_r = float(np.sqrt(np.mean(cpp**2))), float(np.sqrt(np.mean(rust**2)))
            cen_c, cen_r = centroid(cpp, rate), centroid(rust, rate)
            rel_rms = abs(rms_r - rms_c) / max(rms_c, 1e-9)
            rel_cen = abs(cen_r - cen_c) / max(cen_c, 1e-9)
            status = "OK" if rel_rms < STAT_TOLERANCE and rel_cen < STAT_TOLERANCE else "FAIL"
            rows.append((name, kind, f"rms {rms_c:.4f}/{rms_r:.4f} ({rel_rms:.1%})",
                         f"centroïde {cen_c:.0f}/{cen_r:.0f} Hz ({rel_cen:.1%})", "", status))
            continue
        if kind == "voice":
            # Passages par zéro montants (interpolés): même nombre, écart de temps maximal en échantillons.
            ct, rt = crossings(cpp), crossings(rust)
            if len(ct) != len(rt):
                rows.append((name, kind, f"cycles {len(ct)}/{len(rt)}", "", "", "FAIL"))
                continue
            keep = ct > 500  # attente de l'attaque d'enveloppe (1 ms) et du lissage de gain de la voix
            delta = np.abs(ct - rt)[keep]
            err = float(delta.max()) if len(delta) else 0.0
            # 0.02 échantillon = 0.0002 cycle à 220 Hz: erreur de phase < 1e-3 sur un sinus
            rows.append((name, kind, err, float(np.sqrt(np.mean(delta**2))) if len(delta) else 0.0, len(ct),
                         "OK" if err <= 0.02 else "FAIL"))
            continue
        diff = rust - cpp
        max_err = float(np.max(np.abs(diff)))
        rmse = float(np.sqrt(np.mean(diff**2)))
        peak = float(np.max(np.abs(cpp)))
        rows.append((name, kind, max_err, rmse, peak, "OK" if max_err <= THRESHOLD else "FAIL"))
    return rows


def compare_static(out):
    report = []
    for name in ("mappings.txt", "morph.txt"):
        cpp, rust = (out / "cpp" / name).read_text().splitlines(), (out / "rust" / name).read_text().splitlines()
        diffs = [(i + 1, a, b) for i, (a, b) in enumerate(zip(cpp, rust)) if a != b]
        if len(cpp) != len(rust):
            diffs.append(("len", len(cpp), len(rust)))
        report.append((name, len(cpp), diffs))
    draw = []
    for name, count in (("tables.f32", 160 * 33), ("lfo.f32", 13)):
        cpp = np.fromfile(out / "cpp" / name, dtype="<f4").reshape(count, 512)
        rust = np.fromfile(out / "rust" / name, dtype="<f4").reshape(count, 512)
        per = np.max(np.abs(cpp - rust), axis=1)
        bad = int(np.count_nonzero(per > 0))
        draw.append((f"{name} ({bad}/{count} sous-tables différentes, premier écart {int(np.argmax(per > 0)) if bad else '-'})",
                     float(per.max()), float(np.sqrt(np.mean((cpp - rust) ** 2)))))
    for kind in ("wave", "chip", "spec"):
        for osc in range(3):
            cpp = np.fromfile(out / "cpp" / f"draw_{kind}{osc}.f32", dtype="<f4")
            rust = np.fromfile(out / "rust" / f"draw_{kind}{osc}.f32", dtype="<f4")
            draw.append((f"draw_{kind}{osc}", float(np.max(np.abs(cpp - rust))), float(np.sqrt(np.mean((cpp - rust) ** 2)))))
    return report, draw


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=str(ROOT / "target/osc-compare/run"))
    parser.add_argument("--build", default=str(ROOT / "target/osc-compare/build"))
    parser.add_argument("--rate", type=int, default=44100)
    parser.add_argument("--seed", type=int, default=1, help="graine des tables dessinées")
    parser.add_argument("--skip-cpp", action="store_true")
    parser.add_argument("--only", default="")
    args = parser.parse_args()
    out, build = Path(args.out), Path(args.build)
    for sub in ("stimuli", "cpp", "rust", "logs"):
        (out / sub).mkdir(parents=True, exist_ok=True)
    cases = [c for c in build_cases() if args.only in c[0]]
    make_stimuli(out / "stimuli", args.rate)
    spec = out / "spec.txt"
    write_spec(spec, cases, args.seed)
    env = dict(os.environ, MACOSX_DEPLOYMENT_TARGET="13.0")
    os.environ.update(env)
    if not args.skip_cpp:
        if not (build / "CMakeCache.txt").exists():
            run(["cmake", "-S", ROOT / "tools/fidelity", "-B", build, "-DCMAKE_BUILD_TYPE=Release",
                 f"-DODIN_UPSTREAM={ROOT / 'target/fidelity-juce-003/upstream-view'}", f"-DODIN_JUCE={JUCE}",
                 f"-DODIN_TUNING={TUNING}", f"-DODIN_JSON={JSON}", "-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0"],
                out / "logs/cmake.log")
        run(["cmake", "--build", build, "--target", "odin_osc_modules", "--parallel", "4"], out / "logs/cpp-build.log")
        run([build / "odin_osc_modules_artefacts/Release/odin_osc_modules", spec, out / "stimuli", out / "cpp", args.rate],
            out / "logs/cpp-run.log")
    run(["cargo", "build", "--release", "-p", "odin2-core", "--example", "compare_osc_render"], out / "logs/rust-build.log")
    run([ROOT / "target/release/examples/compare_osc_render", spec, out / "stimuli", out / "rust", args.rate], out / "logs/rust-run.log")

    rows = compare_streams(out, cases, args.rate)
    with (out / "comparison.csv").open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("scenario", "type", "max_abs_error|rms", "rmse|centroid", "peak", "status"))
        writer.writerows(rows)
    failures = [r for r in rows if r[-1] != "OK"]
    # Résumé par famille: pire erreur
    families = {}
    for row in rows:
        if row[0].startswith("stat_") or isinstance(row[2], str):
            continue
        key = row[0].split("_")[0]
        worst = families.setdefault(key, [0.0, 0.0, row[0], 0])
        worst[3] += 1
        if row[2] > worst[0]:
            worst[0], worst[1], worst[2] = row[2], row[3], row[0]
    print(f"{'famille':<10}{'n':>5}{'max err':>14}{'RMSE (pire)':>14}  scénario le pire")
    for key, (err, rmse, name, count) in sorted(families.items()):
        print(f"{key:<10}{count:>5}{err:>14.3e}{rmse:>14.3e}  {name}")
    print()
    for row in rows:
        if row[0].startswith("stat_"):
            print(f"{row[0]:<40}{row[2]}  {row[3]}  {row[-1]}")
    report, draw = compare_static(out)
    print()
    for name, count, diffs in report:
        print(f"{name}: {count} lignes, {len(diffs)} différences {diffs[:5]}")
    for name, err, rmse in draw:
        print(f"{name}: max abs {err:.3e}, rmse {rmse:.3e}")
    print(f"\n{len(failures)} scénario(s) hors seuil ({THRESHOLD:g}) sur {len(rows)}")
    for row in sorted((r for r in failures), key=lambda r: -(r[2] if isinstance(r[2], float) else 0))[:60]:
        print("  ", row)


if __name__ == "__main__":
    main()
