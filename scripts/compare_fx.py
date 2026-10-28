#!/usr/bin/env python3
"""Compare les effets Rust (Delay, Chorus, Flanger, Phaser, ZitaReverb, EffectsChain) aux vraies
unités C++ Source/audio/FX compilées avec JUCE (cible odin_fx_modules, tools/fidelity/fx_modules.cpp).

Une seule source de réglages: ce script écrit scenarios.txt, lu à l'identique par le C++ et par
crates/odin2-core/examples/compare_fx_render.rs. Stimuli CSV (frame,input,input_r), 2 s: impulsion,
sinus 440 Hz, balayage 20 Hz→0,45·fs, bruit déterministe, salve.

Exemple (build C++ déjà configuré):
  MACOSX_DEPLOYMENT_TARGET=13.0 python3 scripts/compare_fx.py --out target/fx-compare --rate 44100
"""
import argparse
import csv
import math
import os
from pathlib import Path
import struct
import subprocess
import sys

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
TOLERANCE = 1e-3  # erreur absolue max sur signaux d'amplitude ~0,25


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def write_stimuli(directory, rate):
    directory.mkdir(parents=True, exist_ok=True)
    frames = 2 * rate
    end = min(20000.0, rate * 0.45)
    k = math.log(end / 20.0)
    state = {"seed": 12345}

    def noise(scale):
        # LCG déterministe (Numerical Recipes), identique pour tous les moteurs: lu depuis le CSV.
        state["seed"] = (state["seed"] * 1664525 + 1013904223) & 0xFFFFFFFF
        return scale * ((state["seed"] / 2 ** 31) - 1.0)

    def make(name, left, right):
        with (directory / (name + ".csv")).open("w", newline="") as stream:
            writer = csv.writer(stream)
            writer.writerow(("frame", "input", "input_r"))
            for frame in range(frames):
                t = frame / rate
                writer.writerow((frame, format(f32(left(frame, t)), ".9g"), format(f32(right(frame, t)), ".9g")))

    make("impulse", lambda n, t: 1.0 if n == 0 else 0.0, lambda n, t: 0.0)
    make("impulse2", lambda n, t: 0.8 if n == 0 else 0.0, lambda n, t: -0.5 if n == 0 else 0.0)
    make("sine", lambda n, t: 0.25 * math.sin(2 * math.pi * 440 * t),
         lambda n, t: 0.25 * math.sin(2 * math.pi * 660 * t))
    make("sweep", lambda n, t: 0.25 * math.sin(2 * math.pi * 20.0 * math.expm1(k * t / 2.0) / k * 2.0),
         lambda n, t: 0.25 * math.sin(2 * math.pi * 20.0 * math.expm1(k * (1.0 - t / 2.0)) / k * 2.0))
    make("noise", lambda n, t: noise(0.25), lambda n, t: noise(0.25))
    make("burst", lambda n, t: 0.25 * math.sin(2 * math.pi * 440 * t) if t < 0.2 else 0.0,
         lambda n, t: 0.25 * math.sin(2 * math.pi * 660 * t) if t < 0.2 else 0.0)


# ---------------------------------------------------------------------------------------------
DEFAULTS = {
    "delay": dict(time=0.25, fb=0.45, dry=1.0, wet=0.7, hp=80.0, duck=0.0, pp=0, sync=0, num=3, den=16,
                  bpm=120, mod_time=0, mod_fb=0, mod_hp=0, mod_dry=0, mod_wet=0, reset=1),
    "chorus": dict(amount=0.4, rate=0.15, fb=0.0, drywet=1.0, sync=0, num=3, den=16, bpm=120,
                   mod_freq=0, mod_amount=0, mod_fb=0, mod_drywet=0, reset=1, lforeset=1),
    "flanger": dict(amount=0.3, rate=0.2, fb=0.6, drywet=1.0, sync=0, num=3, den=16, bpm=120,
                    mod_freq=0, mod_amount=0, mod_fb=0, mod_drywet=0, reset=1, lforeset=1),
    "phaser": dict(freq=4000.0, fb=0.25, rate=0.25, amount=0.3, drywet=1.0, radius=1.25, sync=0, num=3,
                   den=16, bpm=120, mod_rate=0, mod_amount=0, mod_freq=0, mod_fb=0, mod_drywet=0,
                   reset=1, lforeset=1),
    "reverb": dict(delay=40.0, mid=2.0, damp=6000.0, eqgain=0.0, eqfreq=1000.0, drywet=0.25,
                   xover=200.0, rtlow=3.0, lowpar=0, reset=1),
    "chain": dict(delay_on=0, delay_time=0.25, delay_fb=0.45, delay_dry=1.0, delay_wet=0.7, delay_hp=80.0,
                  delay_duck=0.0, delay_pp=0, delay_sync=0, delay_num=2, delay_den=5,
                  phaser_on=0, phaser_freq=4000.0, phaser_fb=0.25, phaser_rate=0.25, phaser_amount=0.3,
                  phaser_drywet=1.0, phaser_sync=0, phaser_num=2, phaser_den=5,
                  flanger_on=0, flanger_amount=0.3, flanger_rate=0.2, flanger_fb=0.6, flanger_drywet=1.0,
                  flanger_sync=0, flanger_num=2, flanger_den=5,
                  chorus_on=0, chorus_amount=0.4, chorus_rate=0.15, chorus_fb=0.0, chorus_drywet=1.0,
                  chorus_sync=0, chorus_num=2, chorus_den=5,
                  reverb_on=0, reverb_delay=40.0, reverb_mid=2.0, reverb_damp=6000.0, reverb_eqgain=0.0,
                  reverb_eqfreq=1000.0, reverb_drywet=0.25,
                  pos_delay=0, pos_phaser=1, pos_flanger=2, pos_chorus=3, pos_reverb=4, bpm=120,
                  phaser_reset=0, flanger_reset=0, chorus_reset=0),
}
SIGNALS = ("impulse", "sine", "sweep", "noise")
EVENT_ONLY = ("noteon",)  # clés valables seulement comme événement `clé@trame`


def scenarios():
    rows = []

    def add(name, kind, stimuli, **overrides):
        for stimulus in stimuli:
            params = dict(DEFAULTS[kind])
            unknown = [k for k in overrides if k.split("@")[0] not in params and k.split("@")[0] not in EVENT_ONLY]
            if unknown:
                raise KeyError(f"{name}: clés inconnues {unknown}")
            params.update(overrides)
            rows.append((f"{name}_{stimulus}", kind, stimulus, params))

    # --- Delay ---
    add("delay_base", "delay", SIGNALS + ("impulse2", "burst"))
    add("delay_fb90_wet1", "delay", SIGNALS, time=0.1, fb=0.9, wet=1.0, dry=0.5)
    add("delay_pingpong", "delay", SIGNALS + ("impulse2",), pp=1, time=0.15, fb=0.6)
    add("delay_pingpong_fb0", "delay", ("impulse", "impulse2", "noise"), pp=1, time=0.07, fb=0.0, wet=1.0)
    add("delay_hp20", "delay", SIGNALS, hp=20.0, time=0.05, fb=0.7)
    add("delay_hp100", "delay", SIGNALS, hp=100.0, time=0.05, fb=0.7)
    add("delay_hp500", "delay", SIGNALS, hp=500.0, time=0.05, fb=0.7)
    add("delay_hp5000", "delay", SIGNALS, hp=5000.0, time=0.05, fb=0.7)
    add("delay_duck", "delay", ("sine", "noise", "burst"), duck=0.8, time=0.3, fb=0.5)
    add("delay_duck_pp", "delay", ("sine", "noise", "burst"), duck=1.0, pp=1, time=0.2, fb=0.6)
    add("delay_sync", "delay", ("impulse", "noise", "burst"), sync=1, bpm=100, num=3, den=8, time=0.9)
    add("delay_sync_bpm_change", "delay", ("noise",), sync=1, bpm=120, num=1, den=4, **{"bpm@30000": 150})
    add("delay_retime", "delay", ("impulse", "noise", "sine"), time=0.3, **{"time@30000": 0.1, "time@60000": 0.5})
    add("delay_short", "delay", ("impulse", "noise"), time=0.001, fb=0.8)
    add("delay_long", "delay", ("impulse", "noise"), time=1.5, fb=0.7, wet=1.0)
    add("delay_noreset", "delay", ("impulse", "noise"), time=0.2, reset=0)
    add("delay_mod", "delay", SIGNALS, mod_time=0.2, mod_fb=0.15, mod_hp=0.1, mod_dry=-0.2, mod_wet=0.1,
        time=0.12)
    add("delay_mod_neg", "delay", ("impulse", "noise"), mod_time=-0.3, mod_fb=-0.5, mod_hp=-0.2, mod_dry=0.5,
        mod_wet=-0.9, time=0.12)
    add("delay_params_events", "delay", ("noise",), time=0.2,
        **{"fb@20000": 0.8, "wet@25000": 1.0, "dry@30000": 0.2, "hp@35000": 800.0, "duck@40000": 0.9,
           "pp@50000": 1})

    # --- Chorus ---
    add("chorus_base", "chorus", SIGNALS + ("impulse2",))
    add("chorus_fast", "chorus", SIGNALS, rate=2.5, amount=1.0, fb=0.5)
    add("chorus_negfb", "chorus", SIGNALS, rate=1.0, amount=0.7, fb=-0.7)
    add("chorus_mix", "chorus", SIGNALS, drywet=0.5, amount=0.8, rate=0.6)
    add("chorus_noreset", "chorus", ("sine", "noise"), reset=0, lforeset=0, amount=0.9, rate=0.7)
    add("chorus_reset_only", "chorus", ("sine", "noise"), reset=1, lforeset=0, amount=0.9, rate=0.7)
    add("chorus_sync", "chorus", ("sine", "noise"), sync=1, bpm=100, num=3, den=8)
    add("chorus_mod", "chorus", SIGNALS, mod_freq=0.3, mod_amount=0.2, mod_fb=0.3, mod_drywet=-0.2, rate=1.2)
    add("chorus_events", "chorus", ("noise", "sine"), rate=0.5,
        **{"amount@20000": 0.9, "rate@30000": 3.0, "fb@40000": 0.4, "drywet@50000": 0.6, "lforeset@60000": 1})

    # --- Flanger ---
    add("flanger_base", "flanger", SIGNALS + ("impulse2",))
    add("flanger_fast", "flanger", SIGNALS, rate=3.0, amount=1.0, fb=0.9)
    add("flanger_negfb", "flanger", SIGNALS, fb=-0.8, amount=0.7, rate=0.5)
    add("flanger_mix", "flanger", SIGNALS, drywet=0.5, amount=0.6, rate=1.0)
    add("flanger_noreset", "flanger", ("sine", "noise"), reset=0, lforeset=0, amount=0.8, rate=0.6)
    add("flanger_sync", "flanger", ("sine", "noise"), sync=1, bpm=100, num=3, den=8)
    add("flanger_mod", "flanger", SIGNALS, mod_freq=0.3, mod_amount=0.2, mod_fb=0.2, mod_drywet=-0.2, rate=1.0)
    add("flanger_events", "flanger", ("noise", "sine"), rate=0.5,
        **{"amount@20000": 0.9, "rate@30000": 3.0, "fb@40000": -0.4, "drywet@50000": 0.6})

    # --- Phaser ---
    add("phaser_base", "phaser", SIGNALS + ("impulse2",))
    add("phaser_fast", "phaser", SIGNALS, rate=2.0, amount=1.2, fb=0.9, freq=1000.0)
    add("phaser_mix", "phaser", SIGNALS, drywet=0.4, amount=0.7)
    add("phaser_lowfreq", "phaser", ("sine", "noise"), freq=500.0, amount=0.1, rate=0.4)
    add("phaser_noreset", "phaser", ("sine", "noise"), reset=0, lforeset=0, amount=0.8, rate=0.6)
    add("phaser_reset_only", "phaser", ("sine", "noise"), reset=1, lforeset=0, amount=0.8, rate=0.6)
    add("phaser_radius132", "phaser", ("sine", "noise"), radius=1.32)
    add("phaser_sync", "phaser", ("sine", "noise"), sync=1, bpm=100, num=3, den=8)
    add("phaser_mod", "phaser", SIGNALS, mod_rate=0.3, mod_amount=0.2, mod_freq=0.4, mod_fb=0.3,
        mod_drywet=-0.2, rate=1.0)
    add("phaser_events", "phaser", ("noise", "sine"), rate=0.5,
        **{"amount@20000": 1.0, "rate@30000": 3.0, "fb@40000": 0.9, "freq@45000": 2000.0, "drywet@50000": 0.8})

    # --- Réverb Zita (réponse impulsionnelle stéréo sur 2 s) ---
    add("reverb_base", "reverb", SIGNALS + ("impulse2", "burst"))
    add("reverb_wet1", "reverb", ("impulse", "impulse2", "noise", "sine"), drywet=1.0)
    add("reverb_long_dark", "reverb", ("impulse", "noise", "sine"), mid=6.0, damp=2000.0, drywet=1.0)
    add("reverb_short_bright", "reverb", ("impulse", "noise", "sine"), mid=0.5, damp=12000.0, delay=100.0,
        drywet=0.5)
    add("reverb_predelay0_hors_plage", "reverb", ("impulse", "noise"), delay=0.0, drywet=1.0)
    add("reverb_predelay20", "reverb", ("impulse", "noise"), delay=20.0, drywet=1.0)
    add("reverb_predelay80", "reverb", ("impulse", "noise"), delay=80.0, drywet=1.0)
    add("reverb_eq_boost", "reverb", ("impulse", "noise", "sweep"), eqgain=9.0, eqfreq=3000.0, drywet=1.0)
    add("reverb_eq_cut", "reverb", ("impulse", "noise", "sweep"), eqgain=-9.0, eqfreq=200.0, drywet=1.0)
    add("reverb_lowpar", "reverb", ("impulse", "noise"), lowpar=1, xover=500.0, rtlow=6.0, drywet=1.0)
    add("reverb_noreset", "reverb", ("impulse", "noise"), reset=0, drywet=0.6)
    add("reverb_events", "reverb", ("noise", "impulse"), drywet=1.0,
        **{"mid@20000": 4.0, "damp@30000": 1500.0, "eqgain@40000": 6.0, "eqfreq@45000": 800.0,
           "delay@50000": 80.0, "drywet@60000": 0.3})

    # --- Chaîne FX ---
    all_on = dict(delay_on=1, phaser_on=1, flanger_on=1, chorus_on=1, reverb_on=1, delay_time=0.2,
                  reverb_drywet=0.3)
    add("chain_all_default_order", "chain", SIGNALS + ("impulse2",), **all_on)
    add("chain_all_reverse_order", "chain", SIGNALS, pos_delay=4, pos_phaser=3, pos_flanger=2, pos_chorus=1,
        pos_reverb=0, **all_on)
    add("chain_all_mixed_order", "chain", SIGNALS, pos_delay=2, pos_phaser=0, pos_flanger=4, pos_chorus=1,
        pos_reverb=3, **all_on)
    add("chain_duplicate_positions", "chain", ("noise",), pos_delay=0, pos_phaser=0, pos_flanger=1,
        pos_chorus=1, pos_reverb=2, **all_on)
    add("chain_gap_positions", "chain", ("noise",), pos_delay=3, pos_phaser=7, pos_flanger=1, pos_chorus=2,
        pos_reverb=4, **all_on)
    add("chain_phaser_only", "chain", ("sine", "noise"), phaser_on=1)
    add("chain_delay_only", "chain", ("impulse", "noise"), delay_on=1, delay_time=0.1, delay_pp=1, delay_hp=300.0)
    add("chain_chorus_only", "chain", ("sine", "noise"), chorus_on=1, chorus_rate=0.8)
    add("chain_flanger_only", "chain", ("sine", "noise"), flanger_on=1, flanger_rate=0.5)
    add("chain_reverb_only", "chain", ("impulse", "noise"), reverb_on=1, reverb_drywet=1.0, reverb_mid=3.0)
    add("chain_sync_all", "chain", ("noise", "sine"), delay_on=1, phaser_on=1, flanger_on=1, chorus_on=1,
        delay_sync=1, phaser_sync=1, flanger_sync=1, chorus_sync=1, bpm=90, delay_num=2, delay_den=3,
        phaser_num=3, phaser_den=2, flanger_num=0, flanger_den=1, chorus_num=1, chorus_den=4,
        **{"bpm@30000": 140})
    for fx, extra in (("delay", dict(delay_num=2, delay_den=3)), ("phaser", dict(phaser_num=3, phaser_den=2)),
                      ("flanger", dict(flanger_num=0, flanger_den=1)), ("chorus", dict(chorus_num=1, chorus_den=4))):
        add(f"chain_sync_{fx}_only", "chain", ("noise", "sine"), bpm=90, **{f"{fx}_on": 1, f"{fx}_sync": 1}, **extra,
            **{"bpm@30000": 140})
    add("chain_lfo_reset_on_note", "chain", ("sine", "noise"), phaser_on=1, flanger_on=1, chorus_on=1,
        phaser_reset=1, flanger_reset=1, chorus_reset=1, phaser_rate=0.7, flanger_rate=0.6, chorus_rate=0.9,
        **{"noteon@10000": 1, "noteon@40000": 1})
    add("chain_lfo_reset_flags_off", "chain", ("sine",), phaser_on=1, flanger_on=1, chorus_on=1,
        phaser_reset=0, flanger_reset=1, chorus_reset=0, phaser_rate=0.7, flanger_rate=0.6, chorus_rate=0.9,
        **{"noteon@10000": 1})
    add("chain_params_mix", "chain", ("noise", "sine"), delay_on=1, phaser_on=1, flanger_on=1, chorus_on=1,
        reverb_on=1, delay_time=0.37, delay_fb=0.6, delay_dry=0.8, delay_wet=0.5, delay_hp=200.0, delay_duck=0.5,
        phaser_freq=2500.0, phaser_fb=0.6, phaser_rate=0.9, phaser_amount=0.8, phaser_drywet=0.7,
        flanger_amount=0.5, flanger_rate=0.35, flanger_fb=-0.5, flanger_drywet=0.6,
        chorus_amount=0.7, chorus_rate=0.8, chorus_fb=0.3, chorus_drywet=0.8,
        reverb_delay=70.0, reverb_mid=3.5, reverb_damp=4000.0, reverb_eqgain=4.0, reverb_eqfreq=2500.0,
        reverb_drywet=0.4, pos_delay=1, pos_phaser=4, pos_flanger=0, pos_chorus=2, pos_reverb=3)
    return rows


def fmt(value):
    return repr(float(value)) if not float(value).is_integer() else str(int(value))


def write_scenarios(path, rows):
    with path.open("w") as stream:
        for name, kind, stimulus, params in rows:
            stream.write(f"{name} {kind} {stimulus} " + " ".join(f"{k}={fmt(v)}" for k, v in params.items()) + "\n")


# ---------------------------------------------------------------------------------------------
def run(args, log):
    with log.open("w") as stream:
        stream.write("$ " + " ".join(map(str, args)) + "\n")
        stream.flush()
        result = subprocess.run([str(a) for a in args], stdout=stream, stderr=subprocess.STDOUT, check=False)
    if result.returncode:
        raise RuntimeError(f"échec ({result.returncode}): voir {log}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", type=Path, default=ROOT / "target/fx-compare")
    parser.add_argument("--rate", type=int, default=44100)
    parser.add_argument("--build", type=Path, default=ROOT / "target/fidelity-juce-003/reference-build",
                        help="dossier de build CMake déjà configuré avec ODIN_JUCE")
    parser.add_argument("--skip-cpp-build", action="store_true")
    parser.add_argument("--only", default="", help="sous-chaîne: ne garder que les scénarios dont le nom la contient")
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    (out / "logs").mkdir(exist_ok=True)
    env_note = os.environ.setdefault("MACOSX_DEPLOYMENT_TARGET", "13.0")

    rows = [r for r in scenarios() if args.only in r[0]]
    write_scenarios(out / "scenarios.txt", rows)
    write_stimuli(out / "stimuli", args.rate)

    if not args.skip_cpp_build:
        run(["cmake", "--build", args.build, "--target", "odin_fx_modules", "--parallel", "4"], out / "logs/cpp-build.log")
    exe = next((p for p in sorted(args.build.rglob("odin_fx_modules")) if p.is_file() and os.access(p, os.X_OK)), None)
    if exe is None:
        raise RuntimeError("odin_fx_modules introuvable; construire la cible CMake d'abord")
    run(["cargo", "build", "-p", "odin2-core", "--release", "--example", "compare_fx_render",
         "--target-dir", out / "cargo"], out / "logs/rust-build.log")
    rust = out / "cargo/release/examples/compare_fx_render"
    for engine, binary in (("cpp", exe), ("rust", rust)):
        run([binary, out / "scenarios.txt", out / "stimuli", out / engine, args.rate], out / f"logs/{engine}-render.log")

    results = []
    for name, kind, stimulus, params in rows:
        cpp = np.fromfile(out / "cpp" / f"{name}.f64", dtype="<f8").reshape(-1, 2)
        rust_out = np.fromfile(out / "rust" / f"{name}.f64", dtype="<f8").reshape(-1, 2)
        if cpp.shape != rust_out.shape:
            raise ValueError(f"{name}: longueurs différentes")
        error = np.abs(cpp - rust_out)
        peak = float(np.max(np.abs(cpp)))
        flat = int(np.argmax(error.max(axis=1)))
        results.append({"scenario": name, "kind": kind, "max_abs_error": float(error.max()),
                        "rmse": float(np.sqrt(np.mean((cpp - rust_out) ** 2))),
                        "ref_peak": peak, "ref_rms": float(np.sqrt(np.mean(cpp ** 2))),
                        "worst_frame": flat, "worst_channel": int(np.argmax(error[flat])),
                        "ok": float(error.max()) <= TOLERANCE})
    with (out / "comparison.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(results[0]))
        writer.writeheader()
        writer.writerows(results)

    width = max(len(r["scenario"]) for r in results)
    print(f"{'scénario':<{width}}  {'max_abs':>10}  {'rmse':>10}  {'ref_peak':>9}  {'pire trame':>10}  statut")
    for r in results:
        print(f"{r['scenario']:<{width}}  {r['max_abs_error']:10.3e}  {r['rmse']:10.3e}  {r['ref_peak']:9.4f}  "
              f"{r['worst_frame']:>8}/{r['worst_channel']}  {'OK' if r['ok'] else 'ÉCART'}")
    by_kind = {}
    for r in results:
        by_kind.setdefault(r["kind"], []).append(r)
    print("\nrésumé par genre (max des erreurs, nombre d'écarts > %.0e)" % TOLERANCE)
    for kind, items in by_kind.items():
        print(f"  {kind:<8} scénarios={len(items):3d}  max_abs={max(i['max_abs_error'] for i in items):.3e}  "
              f"écarts={sum(not i['ok'] for i in items)}")
    return 0 if all(r["ok"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
