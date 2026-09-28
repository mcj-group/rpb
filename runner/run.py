#!/usr/bin/env python3

import argparse
import csv
import os
import re
import shlex
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

# presume the the `config.toml` and `run.py` are put in `runner/` folder.
# rpb
#   |_ ...
#   |_ runner/
#      |_ config.toml
#      |_ run.py
#      |_ results/
#      |_ ...
#   |_ ...
#   |_ Cargo.toml
#   |_ README.md

RUNNER_DIR = Path(__file__).resolve().parent
ROOT = RUNNER_DIR.parent
CONFIG = RUNNER_DIR / "config.toml"
RESULTS = RUNNER_DIR / "results"
MULTIQUEUE_BENCHMARKS = {"bfs", "sssp"}
MEAN_PATTERN = re.compile(r"^mean_seconds:\s+([0-9]+(?:\.[0-9]+)?)$")
CSV_FIELDS = (
    "benchmark",
    "mean_seconds",
    "algorithm",
    "workers",
    "rounds",
    "status",
    "input",
)

# error wrapper
class RunnerError(Exception):
    pass

# parses the CLI args
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run RPB benchmarks.")
    parser.add_argument("benchmark", help="benchmark name or 'all'")
    parser.add_argument(
        "-w",
        "--workers",
        type=int,
        help="override the worker count from config.toml",
    )
    parser.add_argument(
        "-r",
        "--rounds",
        type=int,
        help="override the round count from config.toml",
    )
    parser.add_argument(
        "-o",
        "--results-dir",
        type=Path,
        default=RESULTS,
        help="directory for the timestamped CSV file (default: runner/results/)",
    )
    return parser.parse_args()

# loads the configurations from `config.toml`
def load_config() -> dict:
    try:
        with CONFIG.open("rb") as file:
            config = tomllib.load(file)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise RunnerError(f"could not read {CONFIG}: {error}") from error

    for field in ("workers", "rounds"):
        value = config.get(field)
        if value is not None and (not isinstance(value, int) or value < 1):
            raise RunnerError(f"{field} must be a positive integer")
    if not isinstance(config.get("benchmarks"), dict) or not config["benchmarks"]:
        raise RunnerError("config.toml must contain benchmark configurations")
    return config

# returns all benchmark names in the `run all` situation
def select_benchmarks(requested: str, benchmarks: dict) -> list[str]:
    if requested == "all":
        return list(benchmarks)
    if requested not in benchmarks:
        available = ", ".join(benchmarks)
        raise RunnerError(f"unknown benchmark {requested!r}; available: {available}, all")
    return [requested]

# verifies benchmark name and configs, and returns the input path
def validate_benchmark(name: str, config: dict) -> Path:
    input_value = config.get("input")
    if not isinstance(input_value, str) or not input_value:
        raise RunnerError(f"benchmarks.{name}.input must be a nonempty string")
    input_path = Path(os.path.expandvars(input_value)).expanduser()
    if not input_path.is_file():
        raise RunnerError(f"input file not found: {input_path}")

    if name not in MULTIQUEUE_BENCHMARKS and not isinstance(
        config.get("algorithm"), str
    ):
        raise RunnerError(f"benchmarks.{name}.algorithm must be a string")

    buckets = config.get("buckets")
    if buckets is not None and (not isinstance(buckets, int) or buckets < 1):
        raise RunnerError(f"benchmarks.{name}.buckets must be a positive integer")
    return input_path

# prints the cmd executed to stdout
def print_command(stage: str, command: list[str], assignment: str = "") -> None:
    prefix = f"{assignment} " if assignment else ""
    print(f"[{stage}] {prefix}{shlex.join(command)}", flush=True)

# generates run cmd for a benchmark
def command_for(
    name: str,
    config: dict,
    input_path: Path,
    workers: int,
    rounds: int,
) -> list[str]:
    command = [str(ROOT / "target" / "release" / name)]
    if name in MULTIQUEUE_BENCHMARKS:
        command.extend([
            "--threads", str(workers),
            "--rounds", str(rounds),
            "--output", "/dev/null",
        ])
    else:
        command.extend([
            "-a", config["algorithm"],
            "-r", str(rounds),
            "-o", "/dev/null",
        ])
        if "buckets" in config:
            command.extend(["-b", str(config["buckets"])])
    command.append(str(input_path))
    return command

# runs a benchmark and collects `mean_seconds` with Popen
def run(command: list[str], workers: int) -> tuple[int, float | None]:
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(workers)
    print_command("run", command, f"RAYON_NUM_THREADS={workers}")

    try:
        process = subprocess.Popen(
            command,
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            bufsize=1,
        )
    except OSError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1, None

    mean = None
    assert process.stdout is not None
    for line in process.stdout:
        print(line, end="", flush=True)
        match = MEAN_PATTERN.fullmatch(line.strip())
        if match:
            mean = float(match.group(1))

    returncode = process.wait()
    if returncode == 0 and mean is None:
        print("error: benchmark did not report mean_seconds", file=sys.stderr)
        returncode = 1
    return returncode, mean

# runs cmd for benchmark(s)
def execute(
    names: list[str],
    benchmarks: dict,
    input_paths: dict[str, Path],
    workers: int,
    rounds: int,
) -> list[dict]:
    rows = []
    for name in names:
        config = benchmarks[name]
        command = command_for(name, config, input_paths[name], workers, rounds)
        status, mean = run(command, workers)
        rows.append({
            "benchmark": name,
            "mean_seconds": "" if mean is None else f"{mean:.9f}",
            "algorithm": config.get("algorithm", ""),
            "workers": workers,
            "rounds": rounds,
            "status": "success" if status == 0 else "failed",
            "input": input_paths[name],
        })
    return rows

# writes the collected results to a .csv file
def write_results(rows: list[dict], results_dir: Path) -> Path:
    results_dir = results_dir.expanduser().resolve()
    results_dir.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S_%f")
    path = results_dir / f"benchmark_{timestamp}.csv"
    with path.open("x", newline="", encoding="utf-8") as file:
        writer = csv.DictWriter(file, fieldnames=CSV_FIELDS)
        writer.writeheader()
        writer.writerows(rows)
    return path


def main() -> int:
    args = parse_args()
    try:
        config = load_config()
        benchmarks = config["benchmarks"]
        names = select_benchmarks(args.benchmark, benchmarks)
        workers = args.workers if args.workers is not None else config.get("workers")
        if workers is None:
            raise RunnerError("workers must be provided via --workers or config.toml")
        if workers < 1:
            raise RunnerError("workers must be a positive integer")
        rounds = args.rounds if args.rounds is not None else config.get("rounds")
        if rounds is None:
            raise RunnerError("rounds must be provided via --rounds or config.toml")
        if rounds < 1:
            raise RunnerError("rounds must be a positive integer")
        input_paths = {
            name: validate_benchmark(name, benchmarks[name]) for name in names
        }
        rows = execute(names, benchmarks, input_paths, workers, rounds)
        results_path = write_results(rows, args.results_dir)
    except (RunnerError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2

    failures = sum(row["status"] == "failed" for row in rows)
    print(f"Results written to: {results_path}")
    print(f"{len(rows) - failures} succeeded, {failures} failed")
    return int(failures != 0)


if __name__ == "__main__":
    raise SystemExit(main())
