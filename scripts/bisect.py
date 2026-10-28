#!/usr/bin/env python3
"""Bissection C++/Rust: rend un patch avec les mêmes --set dans les deux moteurs.

  bisect.py PRESET.odin [--scenario note] [--set nom=valeur]...
  bisect.py stages PRESET.odin [--scenario note] [--set ...]...

--set nom=valeur: paramètre audio (valeur physique, ex. delay_on=0, fil1_freq=800).
--set noeud/propriété=valeur: propriété non audio (misc, osc, fx, lfo, mod), ex.
misc/dist_on=0, misc/fil3_type=1, mod/source_row_0=0, osc/osc2_type=1.

`stages` retire les étages cumulativement (tout actif, puis sans effets, sans matrice,
sans filtre 3, sans distorsion de voix) pour montrer où l'écart disparaît.
"""
import argparse
import math
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fidelity  # noqa: E402

ROOT = fidelity.ROOT
CPP = ROOT / "target/fidelity-juce-003/reference-build/odin_reference"
RUST = ROOT / "target/release/examples/fidelity_render"
FACTORY = Path("/Users/william.quintal/Project/bascanada/odin2-upstream/assets/Soundbanks/Factory Presets")
EFFECTS_OFF = ["delay_on=0", "reverb_on=0", "chorus_on=0", "flanger_on=0", "phaser_on=0"]
MATRIX_OFF = [f"mod/source_row_{row}=0" for row in range(9)]
STAGES = [
    ("tout actif", []),
    ("sans effets", EFFECTS_OFF),
    ("+ sans matrice", MATRIX_OFF),
    ("+ sans filtre 3", ["misc/fil3_type=1"]),
    ("+ sans distorsion voix", ["misc/dist_on=0"]),
]


def set_arguments(assignments):
    return [item for assignment in assignments for item in ("--set", assignment)]


def render(engine, preset, wav, rate, scenario, seconds, assignments):
    prefix = [str(CPP)] if engine == "cpp" else [str(RUST), "preset"]
    result = subprocess.run(prefix + [str(preset), str(wav), str(rate), scenario, str(seconds)]
                            + set_arguments(assignments), capture_output=True, text=True, timeout=600)
    if result.returncode != 0:
        message = (result.stderr.strip() or result.stdout.strip() or f"code {result.returncode}").splitlines()
        raise RuntimeError(f"{engine}: {message[0] if message else 'échec'}")


def dbfs(value):
    return 20 * math.log10(max(value, 1e-30))


def trial(label, preset, work, index, rate, scenario, seconds, assignments):
    wavs, stats = {}, {}
    for engine in ("cpp", "rust"):
        wavs[engine] = work / f"{index:02d}_{engine}.wav"
        render(engine, preset, wavs[engine], rate, scenario, seconds, assignments)
        stats[engine] = fidelity.metrics(wavs[engine], work / "captures" / engine)
    cpp, rust = stats["cpp"][0], stats["rust"][0]
    comparison = fidelity.compare_audio(stats["rust"], stats["cpp"])
    return {"label": label, "cpp_db": dbfs(cpp["rms"]), "rust_db": dbfs(rust["rms"]),
            "cpp_hz": cpp["centroid_hz"], "rust_hz": rust["centroid_hz"],
            "cpp_flat": cpp["flatness"], "rust_flat": rust["flatness"],
            "l2": comparison.get("envelope_relative_l2"), "rmse": comparison.get("sample_rmse")}


HEADER = ("essai", "RMS C++ dBFS", "RMS Rust dBFS", "Δ RMS dB", "centroïde C++ Hz", "centroïde Rust Hz",
          "platitude C++", "platitude Rust", "L2 enveloppe")


def row(result):
    if "error" in result:
        return (result["label"], "ERREUR: " + result["error"]) + ("",) * 7
    l2 = "n/a (réf. silencieuse)" if result["l2"] is None else f"{result['l2']:.4f}"
    return (result["label"], f"{result['cpp_db']:.2f}", f"{result['rust_db']:.2f}",
            f"{result['rust_db'] - result['cpp_db']:+.2f}", f"{result['cpp_hz']:.1f}", f"{result['rust_hz']:.1f}",
            f"{result['cpp_flat']:.4f}", f"{result['rust_flat']:.4f}", l2)


def table(results):
    rows = [HEADER] + [row(result) for result in results]
    widths = [max(len(line[i]) for line in rows) for i in range(len(HEADER))]
    lines = ["  ".join(cell.ljust(width) if i == 0 else cell.rjust(width)
                       for i, (cell, width) in enumerate(zip(line, widths))) for line in rows]
    lines.insert(1, "  ".join("-" * width for width in widths))
    return "\n".join(lines)


def resolve(path):
    candidate = Path(path)
    if candidate.is_file():
        return candidate.resolve()
    if (FACTORY / path).is_file():
        return FACTORY / path
    raise SystemExit(f"Preset introuvable: {path} (ni chemin direct, ni sous {FACTORY})")


def main():
    argv = sys.argv[1:]
    stages = bool(argv) and argv[0] == "stages"
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("preset", help="fichier .odin (ou chemin relatif au dossier Factory Presets amont)")
    parser.add_argument("--scenario", choices=("note", "pattern"), default="note")
    parser.add_argument("--rate", type=int, default=44100)
    parser.add_argument("--seconds", type=float, default=4.0)
    parser.add_argument("--set", dest="sets", action="append", default=[], metavar="NOM=VALEUR",
                        help="appliqué aux deux moteurs; répétable")
    args = parser.parse_args(argv[1:] if stages else argv)
    for assignment in args.sets:
        if not re.match(r"^[^=]+=.+$", assignment):
            parser.error(f"--set attend nom=valeur: {assignment}")
    for binary in (CPP, RUST):
        if not binary.is_file():
            raise SystemExit(f"Binaire absent: {binary}")
    preset = resolve(args.preset)
    work = ROOT / "target/bisect" / re.sub(r"[^A-Za-z0-9]+", "_", preset.stem).strip("_")
    work.mkdir(parents=True, exist_ok=True)
    if stages:
        plan, removed = [], []
        for label, assignments in STAGES:
            removed += assignments
            plan.append((label, args.sets + removed))
    else:
        plan = [("base", [])] if args.sets else []
        plan.append((" ".join(args.sets) if args.sets else "base", args.sets))
    results = []
    for index, (label, assignments) in enumerate(plan):
        try:
            results.append(trial(label, preset, work, index, args.rate, args.scenario, args.seconds, assignments))
        except (RuntimeError, ValueError, subprocess.TimeoutExpired) as error:
            results.append({"label": label, "error": str(error)})
    print(f"{preset.name}  scénario={args.scenario}  {args.rate} Hz  {args.seconds:g} s  (Δ = Rust − C++)")
    print(table(results))
    return 1 if any("error" in result for result in results) else 0


if __name__ == "__main__":
    sys.exit(main())
