#!/usr/bin/env python3
"""Profile one matching solver per child process (POSIX wait4 peak RSS)."""

import argparse
import csv
import hashlib
import json
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

MASK = (1 << 64) - 1
GENERAL_SOLVERS = (
    "fast-blossom", "gabow-heuristic", "gabow-weighted-approach",
    "mehlhorn-nobahari", "micali-vazirani", "primal-dual",
    "blossom-engine", "sparse-blossom", "duan-scaling",
)
BIPARTITE_SOLVERS = ("hopcroft-karp", "fast-blossom", "hungarian", "primal-dual")
CARDINALITY = {
    "fast-blossom", "gabow-heuristic", "gabow-weighted-approach",
    "mehlhorn-nobahari", "micali-vazirani", "hopcroft-karp",
}
EXPERIMENTAL = {"micali-vazirani"}


def mix(seed, u, v, salt):
    """Stable 64-bit mixing; separate salts keep topology and weights independent."""
    x = (seed ^ (u * 0x9E3779B97F4A7C15) ^
         (v * 0xBF58476D1CE4E5B9) ^ salt) & MASK
    x = ((x ^ (x >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    x = ((x ^ (x >> 27)) * 0x94D049BB133111EB) & MASK
    return x ^ (x >> 31)


def graph_for(family, n, density, weight_range, seed):
    lo, hi = weight_range
    threshold = round(density * 1_000_000)
    edges = []
    for u in range(n):
        for v in range(u + 1, n):
            planted = (u % 2 == 0 and v == u + 1) if family == "general" else (
                u < n // 2 and v == n // 2 + u
            )
            allowed = family == "general" or (u < n // 2 <= v)
            if allowed and (planted or mix(seed, u, v, 0xA5A5) % 1_000_000 < threshold):
                weight = lo + mix(seed, u, v, 0x5A5A) % (hi - lo)
                edges.append({"u": u, "v": v, "weight": weight})
    return {"n": n, "edges": edges}


def graph_bytes(graph):
    return json.dumps(graph, sort_keys=True, separators=(",", ":")).encode() + b"\n"


def run_child(command, timeout_s):
    """Reap this exact child with wait4, so ru_maxrss belongs only to it."""
    started = time.perf_counter_ns()
    with tempfile.TemporaryFile(mode="w+t") as stderr:
        child = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=stderr)
        deadline = time.monotonic() + timeout_s
        timed_out = False
        while True:
            pid, status, usage = os.wait4(child.pid, os.WNOHANG)
            if pid:
                break
            if time.monotonic() >= deadline:
                try:
                    child.kill()
                except ProcessLookupError:
                    pass  # The child exited between the poll and the timeout check.
                _, status, usage = os.wait4(child.pid, 0)
                timed_out = True
                break
            time.sleep(0.005)
        child.returncode = os.waitstatus_to_exitcode(status)
        stderr.seek(0)
        error_text = stderr.read()[-1500:].strip()
    rss_unit = 1 if platform.system() == "Darwin" else 1024
    return {
        "wall_ms": (time.perf_counter_ns() - started) / 1_000_000,
        "peak_rss_bytes": usage.ru_maxrss * rss_unit,
        "exit_code": child.returncode,
        "timed_out": timed_out,
        "error": error_text,
    }


def command_version(command):
    try:
        return subprocess.check_output(command, text=True, stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def scenarios(config):
    for family in ("general", "bipartite"):
        group = config[family]
        for size in group["sizes"]:
            n = size if family == "general" else 2 * size
            for density in group["densities"]:
                for weight_range in group["weight_ranges"]:
                    for run in range(1, config["runs"] + 1):
                        seed = config["seed"] + run - 1
                        yield family, n, density, weight_range, seed, run


def validate_config(config):
    if not isinstance(config["seed"], int) or not 0 <= config["seed"] <= MASK:
        raise ValueError("seed must be a u64")
    if not isinstance(config["runs"], int) or config["runs"] < 1:
        raise ValueError("runs must be positive")
    if config["seed"] + config["runs"] - 1 > MASK:
        raise ValueError("seed plus run count exceeds u64")
    for family in ("general", "bipartite"):
        group = config[family]
        if not all(group[key] for key in ("sizes", "densities", "weight_ranges")):
            raise ValueError(f"{family} needs sizes, densities, and weight_ranges")
        if any(not isinstance(n, int) or n < 2 or (family == "general" and n % 2)
               for n in group["sizes"]):
            raise ValueError(f"{family} sizes must be positive and general sizes even")
        if any(not isinstance(d, (int, float)) or not 0 <= d <= 1
               for d in group["densities"]):
            raise ValueError(f"{family} densities must be in [0,1]")
        if any(len(r) != 2 or not all(isinstance(x, int) for x in r) or r[0] >= r[1]
               for r in group["weight_ranges"]):
            raise ValueError(f"{family} ranges must be integer [low, high) pairs")


def compare_group(rows):
    """Mark disagreements without hiding raw timings or experimental errors."""
    successful = [r for r in rows if r["status"] == "ok"]
    for objective in ("cardinality", "weight"):
        comparable = [r for r in successful if r["objective"] == objective
                      and r["solver"] not in EXPERIMENTAL]
        if not comparable:
            continue
        values = [r["matching_size" if objective == "cardinality" else "total_weight"]
                  for r in comparable]
        expected = values[0]
        all_equal = all(value == expected if objective == "cardinality" else
                        abs(value - expected) <= 1e-9 * max(1, abs(value), abs(expected))
                        for value in values)
        if not all_equal:
            for row in comparable:
                row["status"] = "disagreement"
                row["error"] = f"{objective} values disagree within group: {values}"
    for row in successful:
        if row["solver"] in EXPERIMENTAL:
            reference = next((r for r in successful if r["solver"] == "fast-blossom"), None)
            if reference and row["matching_size"] != reference["matching_size"]:
                row["status"] = "experimental-mismatch"
                row["error"] = "MV cardinality differs from fast-blossom"
        if not row["is_perfect"]:
            row["status"] = "mismatch"
            row["error"] = "planted-perfect graph received a partial matching"
    return rows


def write_outputs(output, records, worker, config_path, timeout_s):
    output.mkdir(parents=True, exist_ok=True)
    (output / "profiles.json").write_text(json.dumps(records, indent=2) + "\n")
    columns = ["family", "n", "m", "requested_density", "observed_density",
               "weight_min", "weight_max_exclusive", "seed", "run", "graph_sha256",
               "solver", "objective", "status", "solver_ms", "wall_ms",
               "peak_rss_bytes", "matching_size", "total_weight", "is_perfect", "error"]
    with (output / "profiles.csv").open("w", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=columns, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(records)
    groups = {}
    for row in records:
        key = (row["family"], row["n"], row["requested_density"],
               row["weight_min"], row["weight_max_exclusive"], row["solver"])
        groups.setdefault(key, []).append(row)
    with (output / "summary.csv").open("w", newline="") as file:
        writer = csv.writer(file)
        writer.writerow(["family", "n", "density", "weight_min", "weight_max_exclusive",
                         "solver", "samples", "ok_samples", "median_solver_ms",
                         "median_wall_ms", "median_peak_rss_bytes"])
        for key, rows in sorted(groups.items()):
            ok = [r for r in rows if r["status"] == "ok"]
            median = lambda column: statistics.median(r[column] for r in ok) if ok else ""
            writer.writerow([*key, len(rows), len(ok), median("solver_ms"),
                             median("wall_ms"), median("peak_rss_bytes")])
    manifest = {
        "schema": "blossom-study-profile-v1",
        "config": str(config_path),
        "config_sha256": hashlib.sha256(config_path.read_bytes()).hexdigest(),
        "driver_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "worker": str(worker),
        "worker_sha256": hashlib.sha256(worker.read_bytes()).hexdigest(),
        "source_commit": command_version(["git", "rev-parse", "HEAD"]),
        "source_dirty": bool(command_version(["git", "status", "--short"])),
        "rustc": command_version(["rustc", "--version"]),
        "platform": platform.platform(),
        "python": sys.version.split()[0],
        "rss_unit_source": "wait4 ru_maxrss; bytes on macOS, KiB on Linux",
        "record_count": len(records),
        "timeout_s": timeout_s,
        "status_counts": {status: sum(r["status"] == status for r in records)
                          for status in sorted({r["status"] for r in records})},
        "measurement": "child peak RSS includes process startup, graph load, solver, validation, and output",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--worker", type=Path, default=Path("target/release/profile_worker"))
    parser.add_argument("--timeout", type=float, default=60.0, help="seconds per solver process")
    args = parser.parse_args()
    if not hasattr(os, "wait4"):
        parser.error("this profiler requires POSIX os.wait4")
    if args.timeout <= 0:
        parser.error("timeout must be positive")
    config = json.loads(args.config.read_text())
    validate_config(config)
    worker = args.worker.resolve()
    if not worker.is_file():
        parser.error(f"worker not found: {worker}; build it with cargo build --release --bin profile_worker")
    args.output.mkdir(parents=True, exist_ok=True)
    if any(args.output.iterdir()):
        parser.error(f"output directory is not empty: {args.output}")
    records = []
    with tempfile.TemporaryDirectory(prefix="blossom-profile-") as temporary:
        graph_path = Path(temporary) / "graph.json"
        result_path = Path(temporary) / "result.json"
        for family, n, density, weight_range, seed, run in scenarios(config):
            graph = graph_for(family, n, density, weight_range, seed)
            content = graph_bytes(graph)
            graph_path.write_bytes(content)
            graph_hash = hashlib.sha256(content).hexdigest()
            scenario_rows = []
            solvers = GENERAL_SOLVERS if family == "general" else BIPARTITE_SOLVERS
            for solver in solvers:
                result_path.unlink(missing_ok=True)
                command = [str(worker), "--solver", solver, "--input", str(graph_path),
                           "--output", str(result_path)]
                observed = run_child(command, args.timeout)
                objective = "cardinality" if solver in CARDINALITY else "weight"
                row = {
                    "family": family, "n": n, "m": len(graph["edges"]),
                    "requested_density": density,
                    "observed_density": len(graph["edges"]) / (
                        n * (n - 1) / 2 if family == "general" else (n // 2) ** 2),
                    "weight_min": weight_range[0], "weight_max_exclusive": weight_range[1],
                    "seed": seed, "run": run, "graph_sha256": graph_hash,
                    "solver": solver, "objective": objective,
                    "status": "timeout" if observed["timed_out"] else "error",
                    "solver_ms": None, "wall_ms": observed["wall_ms"],
                    "peak_rss_bytes": observed["peak_rss_bytes"],
                    "matching_size": None, "total_weight": None, "is_perfect": None,
                    "error": observed["error"],
                }
                if observed["exit_code"] == 0 and result_path.exists():
                    try:
                        result = json.loads(result_path.read_text())
                        row.update(status="ok", solver_ms=result["metadata"]["runtime_ms"],
                                   matching_size=len(result["edges"]),
                                   total_weight=result["total_weight"],
                                   is_perfect=result["is_perfect"], error="")
                    except (OSError, ValueError, KeyError, TypeError) as error:
                        row["error"] = f"could not parse worker result: {error}"
                scenario_rows.append(row)
            records.extend(compare_group(scenario_rows))
            print(f"{family} n={n} density={density} range={weight_range} run={run}: "
                  f"{sum(r['status'] == 'ok' for r in scenario_rows)}/{len(scenario_rows)} ok",
                  flush=True)
    manifest = write_outputs(args.output, records, worker, args.config, args.timeout)
    print(f"Wrote {len(records)} profiles to {args.output}; statuses: {manifest['status_counts']}")
    return int(any(r["status"] not in ("ok", "experimental-mismatch") and
                   r["solver"] not in EXPERIMENTAL for r in records))


if __name__ == "__main__":
    sys.exit(main())
