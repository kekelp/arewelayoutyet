"""Export Criterion estimates and append complete runs to the release archive."""

import argparse
import datetime
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import tomllib

LIBRARIES = {"Taffy main": "taffy", "Morphorm": "morphorm", "Torin": "torin"}
UPSTREAM = "4e59968eda29ceeb0bbaeb3993c8cd38baa3d0a6"


def collect_results(directory):
    results = []
    for metadata_path in sorted(directory.glob("**/new/benchmark.json")):
        metadata = json.loads(metadata_path.read_text())
        library = LIBRARIES.get(metadata.get("value_str"))
        if library is None or metadata.get("group_id") != "PanGUI":
            continue
        estimates = json.loads((metadata_path.parent / "estimates.json").read_text())
        mean = estimates["mean"]
        confidence = mean["confidence_interval"]
        values = [mean["point_estimate"], confidence["lower_bound"], confidence["upper_bound"]]
        if not all(math.isfinite(value) and value > 0 for value in values):
            raise ValueError(f"Invalid estimate in {metadata_path}")
        results.append({"scenario": metadata["function_id"], "library": library,
                        "mean_ns": values[0], "lower_ns": values[1], "upper_ns": values[2]})
    scenarios = {result["scenario"] for result in results if result["library"] == "taffy"}
    templates = list(Path("benchmarks/pangui").glob("*.txt"))
    if len(scenarios) != len(templates):
        raise ValueError("Missing Taffy scenarios, refusing to publish an incomplete run")
    for library in LIBRARIES.values():
        if not any(result["library"] == library for result in results):
            raise ValueError(f"No results for {library}")
    keys = [(result["scenario"], result["library"]) for result in results]
    if len(keys) != len(set(keys)):
        raise ValueError("Duplicate benchmark results")
    return results


def append_run(history, run):
    if history.get("schema") != 1:
        raise ValueError("Unsupported history schema")
    history["runs"] = sorted([entry for entry in history["runs"] if entry["id"] != run["id"]] + [run],
                             key=lambda entry: entry["date"])
    return history


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--criterion", type=Path, default=Path("benchmarks/target/criterion"))
    parser.add_argument("--history", type=Path, default=Path("site/data/history.json"))
    arguments = parser.parse_args()
    results = collect_results(arguments.criterion)
    lock = tomllib.loads(Path("benchmarks/Cargo.lock").read_text())
    versions = {package["name"]: package["version"] for package in lock["package"]
                if package["name"] in LIBRARIES.values()
                and (package["name"] != "taffy" or UPSTREAM in package.get("source", ""))}
    cpu = next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")), platform.machine())
    run = {"id": os.environ.get("GITHUB_RUN_ID", datetime.datetime.now(datetime.timezone.utc).isoformat()),
           "date": datetime.datetime.now(datetime.timezone.utc).isoformat(),
           "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
           "upstream": UPSTREAM, "versions": versions, "cpu": cpu,
           "runner": os.environ.get("RUNNER_OS", platform.system()),
           "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
           "url": f"{os.environ.get('GITHUB_SERVER_URL', 'https://github.com')}/{os.environ.get('GITHUB_REPOSITORY', '')}/actions/runs/{os.environ.get('GITHUB_RUN_ID', '')}",
           "results": results}
    history = json.loads(arguments.history.read_text()) if arguments.history.exists() else {"schema": 1, "runs": []}
    history = append_run(history, run)
    arguments.history.parent.mkdir(parents=True, exist_ok=True)
    arguments.history.write_text(json.dumps(history, indent=2) + "\n")


if __name__ == "__main__":
    main()
