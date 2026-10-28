#!/usr/bin/env python3
"""Banc réel C++/Rust, sans téléchargement et sans référence synthétique."""
import argparse
import cmath
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import shlex
import struct
import subprocess
import sys

PRESETS = ("Bass Crusher", "DeepBass", "Sausage Bassener", "Analog Bass",
           "Darken Nature", "Drifter", "Clouded")
MODULE_SOURCES = ("Source/audio/ADSR.h", "Source/audio/ADSR.cpp",
                  "Source/audio/Filters/BiquadFilter.h", "Source/audio/Filters/BiquadFilter.cpp")
ROOT = Path(__file__).resolve().parents[1]


def make_upstream_view(upstream, view, tuning, json_root):
    """Vue sans copie: dossiers réels et fichiers symlinkés.

    JsonGuiProvider.h inclut ../../libs/json par chemin relatif. En gardant Source/
    en dossiers réels, ce chemin pointe vers cette vue, sans modifier l'amont.
    """
    if view.exists():
        raise RuntimeError(f"Vue amont déjà présente: {view}")
    view.mkdir(parents=True)
    for entry in upstream.iterdir():
        if entry.name in (".git", "libs", "Source"):
            continue
        (view / entry.name).symlink_to(entry)
    for root, directories, files in os.walk(upstream / "Source"):
        relative = Path(root).relative_to(upstream)
        (view / relative).mkdir(parents=True, exist_ok=True)
        for directory in directories:
            (view / relative / directory).mkdir(parents=True, exist_ok=True)
        for name in files:
            (view / relative / name).symlink_to(Path(root) / name)
    libs = view / "libs"
    libs.mkdir()
    for entry in (upstream / "libs").iterdir():
        if entry.name not in ("json", "tuning-library"):
            (libs / entry.name).symlink_to(entry)
    (libs / "json").symlink_to(json_root)
    (libs / "tuning-library").symlink_to(tuning)
    return view


def command(args, log, records, cwd=ROOT):
    log.parent.mkdir(parents=True, exist_ok=True)
    record = {"argv": [str(arg) for arg in args], "cwd": str(cwd), "log": str(log)}
    with log.open("w") as stream:
        stream.write("$ " + shlex.join(record["argv"]) + "\n")
        stream.flush()
        try:
            result = subprocess.run(record["argv"], cwd=cwd, stdout=stream, stderr=subprocess.STDOUT, check=False)
            record["returncode"] = result.returncode
        except OSError as error:
            stream.write(str(error) + "\n")
            record["returncode"] = 127
    records.append(record)
    return record["returncode"] == 0


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def stimuli(directory, rate):
    directory.mkdir(parents=True, exist_ok=True)
    frames = rate
    for scenario in ("impulse", "sine", "sweep"):
        with (directory / (scenario + ".csv")).open("w", newline="") as stream:
            writer = csv.writer(stream)
            writer.writerow(("frame", "input"))
            for frame in range(frames):
                t = frame / rate
                if scenario == "impulse":
                    sample = 1.0 if frame == 0 else 0.0
                elif scenario == "sine":
                    sample = 0.25 * math.sin(2 * math.pi * 440 * t)
                else:
                    start, end = 20.0, min(20000.0, rate * 0.45)
                    k = math.log(end / start)
                    sample = 0.25 * math.sin(2 * math.pi * start * math.expm1(k * t) / k)
                writer.writerow((frame, format(f32(sample), ".9g")))
    for scenario in ("tone", "sweep"):
        with (directory / f"frequency_{scenario}.csv").open("w", newline="") as stream:
            writer = csv.writer(stream)
            writer.writerow(("frame", "input"))
            for frame in range(frames):
                t = frame / rate
                frequency = 220.0 if scenario == "tone" else 55.0 * (1760.0 / 55.0) ** t
                writer.writerow((frame, format(f32(frequency), ".9g")))


def build_modules(upstream, out, records):
    # Les liens conservent les fichiers amont inchangés. Seules les dépendances
    # d'assertion inutilisées numériquement sont remplacées pour ces deux modules.
    shadow = out / "module-sources"
    for relative in MODULE_SOURCES:
        source, target = upstream / relative, shadow / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if target.is_symlink():
            if target.resolve() != source.resolve():
                target.unlink()
        elif target.exists():
            raise RuntimeError(f"Refus de remplacer un fichier non généré: {target}")
        if not target.exists():
            target.symlink_to(source)
    (shadow / "Source/GlobalIncludes.h").write_text(
        "#pragma once\n#include <cassert>\n#include <cmath>\n#define jassert(condition) assert(condition)\n")
    stub = "#pragma once\n#include <cassert>\n#define jassert(condition) assert(condition)\n"
    # ADSR.h inclut "JuceHeader.h", ADSR.cpp "../JuceLibraryCode/…" et BiquadFilter.h
    # "../JuceLibraryCode/…" depuis Source/audio/Filters.
    for directory in (shadow / "Source/JuceLibraryCode", shadow / "Source/audio/JuceLibraryCode"):
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "JuceHeader.h").write_text(stub)
    executable = out / "odin_modules"
    args = shlex.split(os.environ.get("CXX", "c++")) + ["-std=c++17", "-O2", "-fno-fast-math",
        "-I", str(shadow), "-I", str(shadow / "Source/JuceLibraryCode"), str(ROOT / "tools/fidelity/modules.cpp"),
        str(shadow / "Source/audio/ADSR.cpp"),
        str(shadow / "Source/audio/Filters/BiquadFilter.cpp"), "-o", str(executable)]
    return executable if command(args, out / "logs/modules-build.log", records) else None


def read_wav(path):
    data = path.read_bytes()
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise ValueError(f"WAV RIFF attendu: {path}")
    offset, fmt, audio = 12, None, None
    while offset + 8 <= len(data):
        kind, size = struct.unpack_from("<4sI", data, offset)
        start = offset + 8
        if start + size > len(data):
            raise ValueError(f"WAV tronqué: {path}")
        if kind == b"fmt ":
            fmt = struct.unpack_from("<HHIIHH", data, start)
            if fmt[0] == 0xfffe and size >= 40:
                fmt = (struct.unpack_from("<H", data, start + 24)[0],) + fmt[1:]
        elif kind == b"data":
            audio = data[start:start + size]
        offset = start + size + (size & 1)
    if fmt is None or audio is None:
        raise ValueError(f"WAV incomplet: {path}")
    encoding, channels, rate, _, align, bits = fmt
    if channels != 2 or align != channels * bits // 8:
        raise ValueError(f"WAV stéréo sans bourrage requis: {path}")
    if len(audio) % align:
        raise ValueError(f"Trame WAV incomplète: {path}")
    if encoding == 3 and bits == 32:
        values = [item[0] for item in struct.iter_unpack("<f", audio)]
    elif encoding == 1 and bits in (16, 24, 32):
        width = bits // 8
        values = [int.from_bytes(audio[i:i + width], "little", signed=True) / (2 ** (bits - 1))
                  for i in range(0, len(audio), width)]
    else:
        raise ValueError(f"Encodage WAV non pris en charge: {encoding}/{bits}")
    if not values or not all(math.isfinite(value) for value in values):
        raise ValueError(f"WAV vide ou non fini: {path}")
    return rate, values


def fft(values):
    values = [complex(value) for value in values]
    size, j = len(values), 0
    for i in range(1, size):
        bit = size >> 1
        while j & bit:
            j ^= bit
            bit >>= 1
        j ^= bit
        if i < j:
            values[i], values[j] = values[j], values[i]
    width = 2
    while width <= size:
        root = cmath.exp(-2j * math.pi / width)
        for start in range(0, size, width):
            factor = 1.0 + 0j
            for i in range(start, start + width // 2):
                even, odd = values[i], factor * values[i + width // 2]
                values[i], values[i + width // 2] = even + odd, even - odd
                factor *= root
        width *= 2
    return values


def metrics(path, captures):
    rate, samples = read_wav(path)
    frames, bins, fft_size = len(samples) // 2, [0.0] * 1025, 2048
    window = [0.5 - 0.5 * math.cos(2 * math.pi * i / (fft_size - 1)) for i in range(fft_size)]
    windows = 0
    for start in range(0, frames - fft_size + 1, fft_size):
        for channel in (0, 1):
            spectrum = fft([samples[(start + i) * 2 + channel] * window[i] for i in range(fft_size)])
            for i in range(1, len(bins)):
                bins[i] += abs(spectrum[i]) ** 2
            windows += 1
    if windows == 0:
        raise ValueError(f"WAV trop court pour FFT: {path}")
    powers = [value / windows for value in bins[1:]]
    total = sum(powers)
    centroid = sum((i + 1) * rate / fft_size * value for i, value in enumerate(powers)) / max(total, 1e-30)
    flatness = math.exp(sum(math.log(max(value, 1e-30)) for value in powers) / len(powers)) / max(total / len(powers), 1e-30)
    envelope = []
    step = max(1, round(rate * 0.01))
    for start in range(0, frames, step):
        chunk = samples[start * 2:min(frames, start + step) * 2]
        envelope.append(math.sqrt(sum(value * value for value in chunk) / len(chunk)))
    captures.mkdir(parents=True, exist_ok=True)
    with (captures / (path.stem + "_envelope.csv")).open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("seconds", "stereo_rms"))
        writer.writerows((i * step / rate, value) for i, value in enumerate(envelope))
    with (captures / (path.stem + "_spectrum.csv")).open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("hz", "mean_power"))
        writer.writerows(((i + 1) * rate / fft_size, value) for i, value in enumerate(powers))
    return {"rate": rate, "frames": frames, "rms": math.sqrt(sum(value * value for value in samples) / len(samples)),
            "peak": max(map(abs, samples)), "centroid_hz": centroid, "flatness": flatness,
            "envelope": envelope}, samples


def compare_audio(actual, reference):
    a, samples_a = actual
    b, samples_b = reference
    if a["rate"] != b["rate"] or a["frames"] != b["frames"]:
        raise ValueError("Fréquences ou durées différentes: comparaison refusée, aucun réalignement automatique")
    if b["rms"] < 1e-8:
        return {"status": "référence silencieuse: diagnostic requis"}
    envelope_error = math.sqrt(sum((x - y) ** 2 for x, y in zip(a["envelope"], b["envelope"])) /
                               max(sum(y * y for y in b["envelope"]), 1e-30))
    result = {"rms_delta_db": 20 * math.log10(max(a["rms"], 1e-30) / b["rms"]),
              "centroid_relative_error": abs(a["centroid_hz"] - b["centroid_hz"]) / max(b["centroid_hz"], 20),
              "flatness_absolute_error": abs(a["flatness"] - b["flatness"]),
              "envelope_relative_l2": envelope_error,
              "sample_max_error": max(abs(x - y) for x, y in zip(samples_a, samples_b)),
              "sample_rmse": math.sqrt(sum((x - y) ** 2 for x, y in zip(samples_a, samples_b)) / len(samples_a))}
    result["within_screening_tolerances"] = (abs(result["rms_delta_db"]) <= 0.5
        and result["centroid_relative_error"] <= 0.05 and result["flatness_absolute_error"] <= 0.03
        and envelope_error <= 0.1)
    result["status"] = "mesuré (seuils de triage, pas preuve d'identité)"
    return result


def module_names():
    names = ["biquad_" + scenario for scenario in ("impulse", "sine", "sweep")] + ["adsr_gate"]
    for scenario in ("impulse", "sine", "sweep"):
        names += [f"ladder_{mode}_{scenario}" for mode in ("lp24", "lp12", "bp24", "bp12", "hp24", "hp12")]
        names += [f"onepole_{mode}_{scenario}" for mode in ("lp", "hp")]
        names += [f"distortion_{mode}_{scenario}" for mode in ("clamp", "fold", "zero", "sine", "cube")]
    for wave in ("saw", "pulse", "triangle", "sine"):
        names += [f"analog_{wave}_{scenario}" for scenario in ("tone", "sweep")]
    return names


def module_tolerance(name):
    # Seuils fixés avant mesure; voir FIDELITE.md pour leur justification limitée.
    if name.startswith("biquad"):
        return 2e-6
    if name.startswith("adsr"):
        return 2e-3
    if name.startswith(("ladder", "onepole")):
        return 1e-3
    if name.startswith("distortion"):
        return 2e-4
    return 2e-3


def compare_modules(out):
    results = []
    for name in module_names():
        files = [out / kind / (name + ".csv") for kind in ("modules-rust", "modules-cpp")]
        if not all(path.exists() for path in files):
            results.append({"module": name, "status": "référence ou rendu absent"})
            continue
        tables = []
        for path in files:
            with path.open() as stream:
                tables.append([(int(row["frame"]), float(row["input"]), float(row["output"])) for row in csv.DictReader(stream)])
        a, b = tables
        if len(a) != len(b) or not a or any(x[0] != y[0] or f32(x[1]) != f32(y[1]) for x, y in zip(a, b)):
            raise ValueError(f"Stimuli/trames divergents pour {name}")
        errors = [abs(x[2] - y[2]) for x, y in zip(a, b)]
        if not all(math.isfinite(value) for value in errors):
            raise ValueError(f"Module non fini: {name}")
        maximum, rmse = max(errors), math.sqrt(sum(value * value for value in errors) / len(errors))
        threshold = module_tolerance(name)
        results.append({"module": name, "status": "mesuré", "max_abs_error": maximum, "rmse": rmse,
                        "max_abs_tolerance": threshold, "within_tolerance": maximum <= threshold})
    with (out / "modules-comparison.csv").open("w", newline="") as stream:
        fields = ("module", "status", "max_abs_error", "rmse", "max_abs_tolerance", "within_tolerance")
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        writer.writerows(results)
    return results


def resolve_preset(root, name):
    found = sorted(path for path in root.rglob("*.odin")
                   if path.stem == name or path.stem.startswith(name + " ["))
    if len(found) != 1:
        raise ValueError(f"Preset {name!r}: attendu 1 fichier, trouvé {len(found)}: {found}")
    return found[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", required=True, type=Path)
    parser.add_argument("--out", type=Path, default=ROOT / "target/fidelity")
    parser.add_argument("--presets", type=Path, help="Racine banque; défaut assets/Soundbanks/Factory Presets amont")
    parser.add_argument("--reference-dir", type=Path, help="WAV C++/hôte préexistants nommés exactement comme les rendus Rust")
    parser.add_argument("--skip-reference-build", action="store_true", help="Ne pas tenter CMake; l'absence de référence reste explicite")
    parser.add_argument("--juce", type=Path, help="JUCE externe au commit gitlink; active modules audio réels et référence processeur")
    parser.add_argument("--tuning", type=Path, help="tuning-library externe (défaut: sous-module amont)")
    parser.add_argument("--json", type=Path, help="nlohmann/json externe (défaut: sous-module amont)")
    parser.add_argument("--audio-modules-only", action="store_true", help="Avec --juce: ne compiler que les modules audio, pas le processeur")
    parser.add_argument("--rate", type=int, default=44100)
    parser.add_argument("--seconds", type=float, default=4.0)
    args = parser.parse_args()
    if args.rate < 8000 or not math.isfinite(args.seconds) or args.seconds < 3:
        parser.error("--rate >= 8000 et --seconds >= 3 requis")
    upstream, out = args.upstream.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if any(out.iterdir()):
        parser.error("--out doit être vide: empêche toute réutilisation silencieuse d'une ancienne référence")
    records, errors, comparisons, measured = [], [], [], []
    manifest = {"upstream": str(upstream), "rate": args.rate, "seconds": args.seconds,
                "bpm": 120, "velocity": 100, "commands": records, "errors": errors,
                "sources_sha256": {}, "presets_sha256": {}, "comparisons": comparisons}
    try:
        for relative in MODULE_SOURCES + ("Source/GlobalIncludes.h", "Source/PluginProcessorProcess.cpp", "Source/PluginProcessorReadPatch.cpp"):
            manifest["sources_sha256"][relative] = hashlib.sha256((upstream / relative).read_bytes()).hexdigest()
        source_stimuli = out / "stimuli"
        stimuli(source_stimuli, args.rate)
        module_exe = build_modules(upstream, out, records)
        if module_exe:
            if not command([module_exe, source_stimuli, out / "modules-cpp", args.rate], out / "logs/modules-render.log", records):
                errors.append("Rendu modules C++ échoué; voir journal")
        else:
            errors.append("Compilation modules C++ échouée; voir journal")
        reference = None
        if not args.skip_reference_build:
            # Diagnostic direct sur une vraie unité audio non configurée. Ce probe
            # explique notamment JuceHeader.h absent même si CMake bloque d'abord
            # sur un autre sous-module; son échec ne remplace pas la tentative CMake.
            probe = shlex.split(os.environ.get("CXX", "c++")) + [
                "-std=c++17", "-fsyntax-only", str(upstream / "Source/audio/ADSR.cpp")]
            command(probe, out / "logs/reference-source-probe.log", records)
            build = out / "reference-build"
            cmake_upstream = upstream
            if args.juce:
                tuning = (args.tuning or upstream / "libs/tuning-library").resolve()
                json_root = (args.json or upstream / "libs/json").resolve()
                if not (tuning / "include/Tunings.h").is_file() or not (json_root / "include/nlohmann/json.hpp").is_file():
                    raise RuntimeError("--tuning/--json doivent désigner tuning-library (include/Tunings.h) et nlohmann/json (include/nlohmann/json.hpp)")
                cmake_upstream = make_upstream_view(upstream, out / "upstream-view", tuning, json_root)
                manifest["upstream_view"] = str(cmake_upstream)
            cmake_args = ["cmake", "-S", ROOT / "tools/fidelity", "-B", build,
                "-DCMAKE_BUILD_TYPE=Release", f"-DODIN_UPSTREAM={cmake_upstream}"]
            if args.juce:
                cmake_args += [f"-DODIN_JUCE={args.juce.resolve()}", f"-DODIN_TUNING={tuning}", f"-DODIN_JSON={json_root}"]
            if args.audio_modules_only:
                cmake_args.append("-DODIN_AUDIO_MODULES_ONLY=ON")
            configured = command(cmake_args, out / "logs/reference-configure.log", records)
            if configured and args.juce:
                if command(["cmake", "--build", build, "--target", "odin_audio_modules", "--config", "Release", "--parallel", "2"],
                           out / "logs/audio-modules-build.log", records):
                    exe = next((path for path in sorted(build.rglob("odin_audio_modules")) if path.is_file()), None)
                    if exe and exe.is_file() and command([exe, source_stimuli, out / "modules-cpp", args.rate],
                                                         out / "logs/audio-modules-render.log", records):
                        manifest["audio_modules_executable"] = str(exe)
                    else:
                        errors.append("Rendu modules audio JUCE échoué; voir logs/audio-modules-render.log")
                else:
                    errors.append("Compilation modules audio JUCE échouée; voir logs/audio-modules-build.log")
            if configured and not args.audio_modules_only and command(
                    ["cmake", "--build", build, "--target", "odin_reference", "--config", "Release", "--parallel", "2"],
                    out / "logs/reference-build.log", records):
                candidates = [build / "odin_reference", build / "Release/odin_reference"]
                reference = next((path for path in candidates if path.is_file()), None)
            if reference is None and not args.audio_modules_only:
                errors.append("Référence complète C++ indisponible; raison exacte dans logs/reference-*.log")
        cargo_target = out / "cargo"
        if not command(["cargo", "build", "-p", "odin2-core", "--release", "--features", "std", "--example", "fidelity_render",
                        "--target-dir", cargo_target], out / "logs/rust-build.log", records):
            raise RuntimeError("Compilation exemple Rust échouée; voir logs/rust-build.log")
        rust = cargo_target / "release/examples/fidelity_render"
        if not command([rust, "modules", source_stimuli, out / "modules-rust", args.rate], out / "logs/modules-rust.log", records):
            errors.append("Rendu modules Rust échoué; voir journal")
        manifest["modules"] = compare_modules(out)
        preset_root = (args.presets or upstream / "assets/Soundbanks/Factory Presets").resolve()
        for name in PRESETS:
            preset = resolve_preset(preset_root, name)
            manifest["presets_sha256"][str(preset.relative_to(preset_root))] = hashlib.sha256(preset.read_bytes()).hexdigest()
            for scenario in ("note", "pattern"):
                slug = name.replace(" ", "_") + "_" + scenario
                actual = out / "rust" / (slug + ".wav")
                actual.parent.mkdir(parents=True, exist_ok=True)
                if not command([rust, "preset", preset, actual, args.rate, scenario, args.seconds], out / "logs" / (slug + "_rust.log"), records):
                    errors.append(f"Rendu Rust échoué: {slug}")
                    comparisons.append({"preset": name, "scenario": scenario, "status": "rendu Rust échoué"})
                    continue
                a = metrics(actual, out / "captures/rust")
                measured.append({"preset": name, "scenario": scenario, "engine": "rust",
                                 **{k: v for k, v in a[0].items() if k != "envelope"}})
                expected = (args.reference_dir.resolve() if args.reference_dir else out / "cpp") / (slug + ".wav")
                ref_ok = args.reference_dir is not None and expected.is_file()
                if reference and args.reference_dir is None:
                    expected.parent.mkdir(parents=True, exist_ok=True)
                    ref_ok = command([reference, preset, expected, args.rate, scenario, args.seconds], out / "logs" / (slug + "_cpp.log"), records)
                row = {"preset": name, "scenario": scenario}
                if ref_ok:
                    row["reference_sha256"] = hashlib.sha256(expected.read_bytes()).hexdigest()
                    b = metrics(expected, out / "captures/cpp")
                    measured.append({"preset": name, "scenario": scenario, "engine": "cpp-or-import",
                                     **{k: v for k, v in b[0].items() if k != "envelope"}})
                    row.update(compare_audio(a, b))
                else:
                    row["status"] = "référence absente: fidélité non mesurée"
                comparisons.append(row)
    except (OSError, ValueError, RuntimeError) as error:
        errors.append(str(error))
    manifest["metrics"] = measured
    (out / "report.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False, allow_nan=False) + "\n")
    if measured:
        with (out / "metrics.csv").open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(measured[0]))
            writer.writeheader()
            writer.writerows(measured)
    with (out / "comparison.csv").open("w", newline="") as stream:
        fields = ("preset", "scenario", "status", "rms_delta_db", "centroid_relative_error", "flatness_absolute_error",
                  "envelope_relative_l2", "sample_max_error", "sample_rmse", "within_screening_tolerances", "reference_sha256")
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        writer.writerows(comparisons)
    report = ["# Rapport de mesures Odin 2", "", "Rapport généré à partir des exécutions consignées; aucune référence remplacée par Rust.",
              "", "| Preset | Scénario | État |", "|---|---|---|"]
    report.extend(f"| {row['preset']} | {row['scenario']} | {row['status']} |" for row in comparisons)
    report += ["", "## Diagnostics", ""] + (["- " + error for error in errors] or ["Aucune erreur d'exécution enregistrée."])
    report += ["", "Les seuils de triage et limites de couverture sont documentés dans FIDELITE.md.", "Les valeurs numériques sont dans comparison.csv, modules-comparison.csv et report.json."]
    (out / "rapport.md").write_text("\n".join(report) + "\n")
    print(out / "rapport.md")
    complete = len(comparisons) == len(PRESETS) * 2 and all("within_screening_tolerances" in row for row in comparisons)
    within = complete and all(row["within_screening_tolerances"] for row in comparisons)
    modules = manifest.get("modules", [])
    modules_ok = len(modules) == len(module_names()) and all(row.get("within_tolerance", False) for row in modules)
    return 0 if not errors and within and modules_ok else 1


if __name__ == "__main__":
    sys.exit(main())
