#!/usr/bin/env python3
"""Compare les filtres Rust aux vrais filtres Odin 2 (JUCE) : Diode, Korg35, SEM, Comb, Formant,
RingMod et la chaîne PresetFilter (OdinFilterBase::update).

Le C++ est compilé sans CMake: on réutilise les objets de la cible odin_audio_modules déjà
construite (dossier de build du banc), seul tools/fidelity/filters_modules.cpp est recompilé.

  python3 scripts/compare_filters.py [--out DIR] [--build DIR] [--rate 44100] [--skip-cpp] [--only SUBSTR]
"""
import argparse
import csv
import math
import os
import shlex
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from fidelity import stimuli as write_stimuli  # noqa: E402

THRESHOLD = 1e-3  # erreur absolue max pour des signaux d'amplitude 0.25 (impulsion: 1.0)


def fmt(value):
    return format(value, "g").replace("-", "m").replace(".", "p")


def build_cases():
    cases = []

    def add(name, kind, **kw):
        cases.append((name, kind, kw))

    # --- Diode (module nu; od = overdrive direct, soit 2 x saturation)
    for freq in (100, 1000, 8000):
        for res in (0, 0.5, 0.9):
            add(f"diode_f{fmt(freq)}_r{fmt(res)}", "diode", freq=freq, res=res, od=0)
    add("diode_analogbass", "diode", freq=121, res=0.224, od=0.6)
    for freq, res, od in ((100, 0.5, 0.6), (1000, 0.5, 1.0), (1000, 0.5, 1.7), (8000, 0.9, 0.6),
                          (20, 0.5, 0), (20000, 0.5, 0), (100, 1.0, 0)):
        add(f"diode_f{fmt(freq)}_r{fmt(res)}_od{fmt(od)}", "diode", freq=freq, res=res, od=od)
    # --- Korg35 LP / HP
    for lp in (1, 0):
        tag = "lp" if lp else "hp"
        for freq in (100, 1000, 8000):
            for res in (0, 0.5, 0.9, 1.0):
                add(f"korg{tag}_f{fmt(freq)}_r{fmt(res)}", "korg", lp=lp, freq=freq, res=res, od=0)
        for od in (0.6, 1.5):
            add(f"korg{tag}_f1000_r0p5_od{fmt(od)}", "korg", lp=lp, freq=1000, res=0.5, od=od)
    # --- SEM (plusieurs transitions)
    for trans in (-1, -0.5, 0, 0.5, 1):
        for freq, res in ((100, 0), (100, 0.5), (1000, 0.9), (8000, 0.5), (1000, 1.0)):
            add(f"sem_t{fmt(trans)}_f{fmt(freq)}_r{fmt(res)}", "sem", trans=trans, freq=freq, res=res)
    # --- Comb (polarité + et -); pos=1 positif
    for pos in (1, 0):
        for freq in (100, 558, 2000):
            for res in (0, 0.5, 0.9):
                add(f"comb_{'p' if pos else 'n'}_f{fmt(freq)}_r{fmt(res)}", "comb", pos=pos, freq=freq, res=res)
    add("comb_p_f558_r0p5_noreset", "comb", pos=1, freq=558, res=0.5, noreset=1)
    add("comb_n_f558_r0p5_noreset", "comb", pos=0, freq=558, res=0.5, noreset=1)
    # --- Formant (voyelles / transition)
    for vl, vr in ((0, 1), (3, 4), (5, 7), (2, 2), (0, 2), (6, 1)):
        for trans in (0, 0.25, 0.5, 1):
            add(f"formant_{vl}{vr}_t{fmt(trans)}", "formant", vl=vl, vr=vr, trans=trans)

    # --- Chaîne PresetFilter (valeurs stationnaires) ---
    base = dict(freq=1000, res=0.5, sat=0.3)
    mods = dict(freq=500, res=0.4, sat=0.2, kbd=0.4, note=30, velamt=0.3, velocity=90, envamt=0.5, env=0.6,
                mfreq=0.05, mres=0.1, msat=0.05, menv=0.1, mvel=0.1, mkbd=0.1, gain=-6, mgain=0.3)
    types = {"lp24": 2, "lp12": 3, "bp24": 4, "bp12": 5, "hp24": 6, "hp12": 7, "sem": 15, "diode": 19,
             "korglp": 24, "korghp": 25}
    for tag, ftype in types.items():
        extra = {"trans": -0.5} if tag == "sem" else {}
        add(f"preset_{tag}_plain", "preset", type=ftype, **base, **extra)
        add(f"preset_{tag}_mods", "preset", type=ftype, **{**mods, **extra}, **({"mtrans": 0.2} if tag == "sem" else {}))
        add(f"preset_{tag}_negmods", "preset", type=ftype,
            **{**mods, **extra, "mkbd": -0.5, "kbd": 0.2, "mvel": -0.6, "velamt": 0.3, "mfreq": -0.05, "mres": -0.2, "msat": -0.3})
    add("preset_diode_analogbass", "preset", type=19, freq=121, res=0.224, sat=0.3, envamt=0.585, env=0.7,
        note=40, velocity=100)
    add("preset_diode_analogbass_env1", "preset", type=19, freq=121, res=0.224, sat=0.3, envamt=0.585, env=1.0,
        note=40, velocity=100)
    add("preset_diode_analogbass_env0", "preset", type=19, freq=121, res=0.224, sat=0.3, envamt=0.585, env=0.0,
        note=40, velocity=100)
    add("preset_sem_trans1", "preset", type=15, trans=1, mtrans=0.3, freq=800, res=0.6)
    add("preset_sem_modtrans_neg", "preset", type=15, trans=0.1, mtrans=-0.3, freq=800, res=0.6)
    # Comb: filtre 3 du preset « Sausage Bassener » (558 Hz, kbd 0.472, +12 dB), note MIDI brute
    for pol in (0, 1):
        add(f"preset_comb_sausage_pol{pol}", "preset", type=30, freq=558, res=0.3, kbd=0.472, note=48, velocity=100,
            gain=12, pol=pol)
    add("preset_comb_mods", "preset", type=30, freq=300, res=0.6, kbd=0.3, note=30, velamt=0.3, velocity=90,
        envamt=0.5, env=0.6, mfreq=0.05, mres=0.1, menv=0.1, mvel=0.1, mkbd=0.1, gain=-3, mgain=0.2)
    add("preset_comb_negmods", "preset", type=30, freq=300, res=0.6, kbd=0.3, note=30, velamt=0.3, velocity=90,
        mkbd=-0.5, mvel=-0.6, mfreq=-0.05, mres=-0.2)
    add("preset_comb_highnote", "preset", type=30, freq=2000, res=0.5, kbd=1.0, note=100, pol=1)
    # Formant
    for vl, vr in ((0, 1), (0, 2), (3, 6)):
        add(f"preset_formant_{vl}{vr}", "preset", type=35, vl=vl, vr=vr, ftrans=0.3)
    add("preset_formant_mods", "preset", type=35, vl=0, vr=2, ftrans=0.2, velamt=0.5, velocity=100, envamt=0.4,
        env=0.5, mtrans=0.1)
    # Ring mod
    add("preset_ring_plain", "preset", type=40, freq=440, ring=1.0)
    add("preset_ring_half", "preset", type=40, freq=1000, ring=0.5)
    add("preset_ring_mods", "preset", type=40, freq=300, ring=0.8, kbd=0.4, note=30, velamt=0.3, velocity=90,
        envamt=0.5, env=0.6, mfreq=0.05, mkbd=0.1, mvel=0.1, menv=0.1, mring=0.1, gain=-6)
    add("preset_ring_negmods", "preset", type=40, freq=300, ring=0.8, kbd=0.4, note=30, mkbd=-0.6, mring=-0.3,
        mvel=-0.2, velamt=0.3, velocity=90)
    add("preset_ring_highfreq", "preset", type=40, freq=15000, ring=1.0, kbd=1.0, note=60)
    # Bornes: résonance modulée au-delà de 1, fréquence bornée à 20 Hz / 20 kHz par la chaîne de modulation.
    for tag, ftype in (("lp24", 2), ("hp12", 7), ("sem", 15), ("diode", 19), ("korglp", 24), ("korghp", 25)):
        extra = {"trans": 0.2} if tag == "sem" else {}
        add(f"preset_{tag}_clampres", "preset", type=ftype, freq=1000, res=0.9, mres=0.5, sat=0.6, msat=0.4, **extra)
        add(f"preset_{tag}_clamplow", "preset", type=ftype, freq=60, res=0.5, mfreq=-0.9, kbd=0.2, note=0, **extra)
        add(f"preset_{tag}_clamphigh", "preset", type=ftype, freq=8000, res=0.5, envamt=1.0, env=1.0, **extra)
    add("preset_diode_gain_low", "preset", type=19, freq=300, res=0.4, gain=-30, mgain=0.5)
    add("preset_diode_gain_mute", "preset", type=19, freq=300, res=0.4, gain=-60, mgain=0.5)
    add("preset_comb_gain12", "preset", type=30, freq=558, res=0.3, gain=12, mgain=0.4, pol=1)
    add("preset_comb_cancel_gate", "preset", type=30, freq=500, res=0.4, mfreq=0.2, menv=-0.2)
    add("preset_lp24_cancel_gate", "preset", type=2, freq=500, res=0.4, mfreq=0.2, menv=-0.2)
    # BiquadAllpass (phaser amont) et résonateur (via formant)
    for freq in (100, 1000, 8000):
        for radius in (1.05, 1.32, 2.0):
            add(f"allpass_f{fmt(freq)}_r{fmt(radius)}", "allpass", freq=freq, radius=radius)
    return cases


def write_cases(path, cases):
    with path.open("w") as stream:
        for name, kind, kw in cases:
            stream.write(" ".join([name, kind] + [f"{k}={v}" for k, v in kw.items()]) + "\n")


def run(args, **kw):
    result = subprocess.run([str(a) for a in args], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, **kw)
    if result.returncode:
        lines = [l for l in result.stdout.splitlines() if "error" in l or "Error" in l or "panicked" in l or "undefined" in l]
        sys.stderr.write("\n".join(lines[:30] or result.stdout.splitlines()[-30:]) + "\n")
        raise SystemExit(f"échec: {shlex.join(str(a) for a in args)[:200]}")
    return result.stdout


# Unités amont recompilées sans contraction FMA (voir build_cpp).
NOFMA_UNITS = [
    "Filters/BiquadAllpass", "Filters/BiquadFilter", "Filters/BiquadResonator", "Filters/CombFilter",
    "Filters/DiodeFilter", "Filters/FormantFilter", "Filters/Korg35Filter", "Filters/LadderFilter",
    "Filters/OdinFilterBase", "Filters/SEMFilter12", "Filters/VAOnePoleFilter", "FX/RingModulator",
    "Oscillators/Oscillator", "Oscillators/WavetableOsc1D",
]


def build_cpp(build, out, fma):
    src = ROOT / "tools/fidelity/filters_modules.cpp"
    obj = out / "filters_modules.o"
    exe = out / "odin_filters_modules"
    dep = build / "CMakeFiles/odin_audio_modules.dir"
    flags = {}
    for line in (dep / "flags.make").read_text().splitlines():
        if " = " in line:
            key, value = line.split(" = ", 1)
            flags[key] = shlex.split(value)
    env = dict(os.environ, MACOSX_DEPLOYMENT_TARGET="13.0")
    base = ["c++", *flags["CXX_DEFINES"], *flags["CXX_INCLUDES"], *flags["CXX_FLAGS"]]
    # clang arm64 fusionne `a*b+c` en FMA par défaut (-ffp-contract=on), rustc jamais. Les erreurs
    # d'arrondi ainsi créées (2e-7 relatif sur un exposant de 47 demi-tons) s'accumulent en phase
    # sur 1 s. Sans --fma, notre unité et les unités filtres/oscillateur ci-dessus sont recompilées
    # avec -ffp-contract=off pour que la référence s'arrondisse comme le Rust.
    contract = [] if fma else ["-ffp-contract=off"]
    run([*base, *contract, "-c", src, "-o", obj], cwd=build, env=env)
    upstream = next(Path(a[2:]) for a in flags["CXX_INCLUDES"] if a.startswith("-I") and a.endswith("/Source"))
    replaced = {}
    if not fma:
        (out / "nofma").mkdir(exist_ok=True)
        for unit in NOFMA_UNITS:
            unit_obj = out / "nofma" / (unit.replace("/", "_") + ".o")
            run([*base, "-ffp-contract=off", "-c", upstream / "audio" / (unit + ".cpp"), "-o", unit_obj], cwd=build, env=env)
            replaced[f"/Source/audio/{unit}.cpp.o"] = str(unit_obj)
    link = shlex.split((dep / "link.txt").read_text())
    swapped = []
    for arg in link:
        if arg.endswith("audio_modules.cpp.o") and "upstream-view" not in arg:
            arg = str(obj)
        else:
            for suffix, unit_obj in replaced.items():
                if arg.endswith(suffix):
                    arg = unit_obj
        swapped.append(arg)
    swapped[swapped.index("-o") + 1] = str(exe)
    run(swapped, cwd=build, env=env)
    return exe


def read(path):
    rows = list(csv.reader(path.open()))[1:]
    return [float(r[2]) for r in rows]


def compare(out, cases, only):
    rows = []
    for name, kind, kw in cases:
        if only and only not in name:
            continue
        for scenario in ("impulse", "sine", "sweep"):
            rust = read(out / "rust" / f"{name}_{scenario}.csv")
            cpp = read(out / "cpp" / f"{name}_{scenario}.csv")
            assert len(rust) == len(cpp)
            diffs = [r - c for r, c in zip(rust, cpp)]
            max_abs = max(abs(d) for d in diffs)
            at = max(range(len(diffs)), key=lambda i: abs(diffs[i]))
            rmse = math.sqrt(sum(d * d for d in diffs) / len(diffs))
            peak = max(abs(c) for c in cpp)
            rms_ref = math.sqrt(sum(c * c for c in cpp) / len(cpp))
            rows.append(dict(case=name, scenario=scenario, max_abs=max_abs, at_frame=at, rmse=rmse,
                             cpp_peak=peak, cpp_rms=rms_ref, rust_peak=max(abs(r) for r in rust),
                             ok=max_abs <= THRESHOLD))
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, default=ROOT / "target/fidelity-filters")
    parser.add_argument("--build", type=Path, default=ROOT / "target/fidelity-juce-003/reference-build")
    parser.add_argument("--rate", type=int, default=44100)
    parser.add_argument("--skip-cpp", action="store_true", help="réutilise cpp/ déjà rendu")
    parser.add_argument("--only", default="", help="ne compare que les cas dont le nom contient ce texte")
    parser.add_argument("--worst", type=int, default=0, help="n'affiche que les N pires lignes")
    parser.add_argument("--fma", action="store_true", help="garde les objets amont tels que compilés (FMA clang arm64)")
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    cases = build_cases()
    write_cases(out / "cases.txt", cases)
    write_stimuli(out / "stimuli", args.rate)
    if not args.skip_cpp:
        exe = build_cpp(args.build, out, args.fma)
        run([exe, out / "cases.txt", out / "stimuli", out / "cpp", args.rate])
    run(["cargo", "run", "--release", "-q", "-p", "odin2-core", "--example", "compare_filters_render", "--",
         out / "cases.txt", out / "stimuli", out / "rust", args.rate], cwd=ROOT)
    rows = compare(out, cases, args.only)
    with (out / "comparison.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    shown = sorted(rows, key=lambda r: -r["max_abs"])[: args.worst] if args.worst else rows
    print(f"{'cas':44} {'scén.':8} {'max|Δ|':>10} {'RMSE':>10} {'pic C++':>9} {'rms C++':>9}")
    for r in shown:
        flag = "" if r["ok"] else "  <-- ÉCART"
        print(f"{r['case']:44} {r['scenario']:8} {r['max_abs']:10.3e} {r['rmse']:10.3e} {r['cpp_peak']:9.4f} {r['cpp_rms']:9.4f}{flag}")
    bad = [r for r in rows if not r["ok"]]
    print(f"\n{len(rows)} comparaisons, {len(bad)} au-dessus de {THRESHOLD:g}; "
          f"pire max|Δ| = {max(r['max_abs'] for r in rows):.3e}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
