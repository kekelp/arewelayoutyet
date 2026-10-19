"""Download and replace a single compressed GitHub release asset."""

import argparse
import gzip
import json
import os
from pathlib import Path
import shutil
import subprocess

TAG = "benchmark-data"
ASSET = "history.json.gz"
HISTORY = Path("site/data/history.json")
ARCHIVE = Path(".history")


def gh(*arguments):
    return subprocess.check_output(["gh", *arguments], text=True).strip()


def find_release(repository):
    pages = json.loads(gh("api", "--paginate", "--slurp", f"repos/{repository}/releases?per_page=100"))
    return next((release for page in pages for release in page if release["tag_name"] == TAG), None)


def validate_history(data):
    if data.get("schema") != 1 or not isinstance(data.get("runs"), list):
        raise ValueError("Unsupported or invalid benchmark history")
    identifiers = set()
    for run in data["runs"]:
        if not isinstance(run.get("id"), str) or run["id"] in identifiers:
            raise ValueError("Invalid or duplicate run ID")
        identifiers.add(run["id"])
        if not isinstance(run.get("date"), str) or not isinstance(run.get("results"), list) or not run["results"]:
            raise ValueError("Invalid benchmark run")
    return data


def download(repository):
    release = find_release(repository)
    ARCHIVE.mkdir(exist_ok=True)
    if release is None:
        if HISTORY.exists():
            raise ValueError("Refusing to overwrite local history during bootstrap")
        HISTORY.parent.mkdir(parents=True, exist_ok=True)
        HISTORY.write_text(json.dumps({"schema": 1, "runs": []}) + "\n")
        return
    if not any(asset["name"] == ASSET for asset in release["assets"]):
        raise ValueError("The data release exists but its history asset is missing. Restore the backup before running again")
    gh("release", "download", TAG, "--repo", repository, "--pattern", ASSET, "--dir", str(ARCHIVE), "--clobber")
    archive = ARCHIVE / ASSET
    content = gzip.decompress(archive.read_bytes()).decode("utf-8")
    validate_history(json.loads(content))
    shutil.copyfile(archive, ARCHIVE / "previous-history.json.gz")
    HISTORY.parent.mkdir(parents=True, exist_ok=True)
    HISTORY.write_text(content)


def upload(repository):
    history = validate_history(json.loads(HISTORY.read_text()))
    if not history["runs"]:
        raise ValueError("Refusing to publish an empty history")
    previous = ARCHIVE / "previous-history.json.gz"
    if previous.exists():
        previous_history = validate_history(json.loads(gzip.decompress(previous.read_bytes())))
        if not {run["id"] for run in previous_history["runs"]}.issubset({run["id"] for run in history["runs"]}):
            raise ValueError("Refusing to publish history that drops existing runs")
    ARCHIVE.mkdir(exist_ok=True)
    archive = ARCHIVE / ASSET
    archive.write_bytes(gzip.compress(HISTORY.read_bytes(), mtime=0))
    release = find_release(repository)
    if release is None:
        gh("release", "create", TAG, str(archive), "--repo", repository,
           "--target", os.environ["GITHUB_SHA"], "--title", "Benchmark history",
           "--notes", "Machine-generated layout benchmark history. The daily workflow replaces history.json.gz. Backups are kept as Actions artifacts.",
           "--prerelease", "--latest=false")
    else:
        gh("release", "upload", TAG, str(archive), "--repo", repository, "--clobber")
    verification = ARCHIVE / "verification"
    verification.mkdir(exist_ok=True)
    gh("release", "download", TAG, "--repo", repository, "--pattern", ASSET, "--dir", str(verification))
    if (verification / ASSET).read_bytes() != archive.read_bytes():
        raise ValueError("Uploaded history did not match the local archive")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("operation", choices=["download", "upload"])
    arguments = parser.parse_args()
    repository = os.environ["GITHUB_REPOSITORY"]
    {"download": download, "upload": upload}[arguments.operation](repository)


if __name__ == "__main__":
    main()
