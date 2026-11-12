#!/usr/bin/env python3
"""Recreate and reduce false-infeasibility profile rows with the Rust reducer."""

import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

import profile_solvers as profile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles", type=Path, required=True)
    parser.add_argument("--reducer", type=Path,
                        default=Path("target/release/minimize_profile_failure"))
    args = parser.parse_args()
    rows = json.loads(args.profiles.read_text())
    reducer = args.reducer.resolve()
    if not reducer.is_file():
        parser.error("build reducer with cargo build --release --bin minimize_profile_failure")
    output = args.profiles.parent / "failures"
    output.mkdir(exist_ok=True)
    failures = [row for row in rows if row["status"] == "error" and
                "graph does not have a perfect matching" in row["error"]]
    with tempfile.TemporaryDirectory(prefix="blossom-reduce-") as directory:
        graph_path = Path(directory) / "graph.json"
        for row in failures:
            graph = profile.graph_for(row["family"], row["n"], row["requested_density"],
                                      [row["weight_min"], row["weight_max_exclusive"]], row["seed"])
            content = profile.graph_bytes(graph)
            digest = hashlib.sha256(content).hexdigest()
            if digest != row["graph_sha256"]:
                raise ValueError("profile graph hash did not reproduce")
            graph_path.write_bytes(content)
            filename = (f"{row['solver']}-n{row['n']}-d{row['requested_density']}-"
                        f"w{row['weight_min']}_{row['weight_max_exclusive']}-run{row['run']}.json")
            artifact_path = output / filename
            subprocess.run([str(reducer), "--solver", row["solver"], "--input", str(graph_path),
                            "--output", str(artifact_path)], check=True)
            artifact = json.loads(artifact_path.read_text())
            artifact["seed"] = row["seed"]
            artifact["original_graph_sha256"] = digest
            artifact["profile_row"] = {
                "family": row["family"], "n": row["n"],
                "requested_density": row["requested_density"],
                "weight_range": [row["weight_min"], row["weight_max_exclusive"]],
                "run": row["run"], "error": row["error"],
            }
            artifact_path.write_text(json.dumps(artifact, indent=2) + "\n")
            print(f"saved {artifact_path}", flush=True)


if __name__ == "__main__":
    main()
