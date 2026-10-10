"""Same measurement schema for Electron and Rust. Dependency installation excluded.

macOS /usr/bin/time -l and Linux /usr/bin/time -v measure peak process RSS.
On Windows RSS is unavailable in this harness and explicitly recorded as null.
"""
import argparse
import datetime
import hashlib
import json
import platform
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--implementation",choices=["electron","rust"],required=True)
parser.add_argument("--metric",required=True)
parser.add_argument("--runs",type=int,default=3)
parser.add_argument("--output",type=Path,required=True)
parser.add_argument("--input",type=Path)
parser.add_argument("--artifact",type=Path)
parser.add_argument("--cwd",type=Path)
parser.add_argument("command",nargs=argparse.REMAINDER)
args = parser.parse_args()
command = args.command[1:] if args.command[:1]==["--"] else args.command
args.output.parent.mkdir(parents=True,exist_ok=True)
samples = []
for run in range(args.runs):
    timed = ["/usr/bin/time","-l",*command] if sys.platform=="darwin" else ["/usr/bin/time","-v",*command] if sys.platform.startswith("linux") else command
    start = time.perf_counter()
    result = subprocess.run(timed,capture_output=True,text=True,cwd=args.cwd)
    elapsed = (time.perf_counter()-start)*1000
    args.output.with_suffix(f".run{run+1}.stdout.log").write_text(result.stdout)
    args.output.with_suffix(f".run{run+1}.stderr.log").write_text(result.stderr)
    match = re.search(r"(\d+)\s+maximum resident set size",result.stderr) if sys.platform=="darwin" else re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)",result.stderr)
    rss = int(match.group(1))*(1 if sys.platform=="darwin" else 1024) if match else None
    samples.append({"elapsed_ms":elapsed,"peak_rss_bytes":rss,"exit_code":result.returncode})
    print(f"{args.implementation} {args.metric} {run+1}/{args.runs}: {elapsed/1000:.3f}s, exit {result.returncode}",flush=True)
    if result.returncode: break
report = {"schema_version":1,"implementation":args.implementation,"commit":subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),
    "environment":{"os":platform.platform(),"architecture":platform.machine(),"timestamp":datetime.datetime.now().astimezone().isoformat()},
    "metric":args.metric,"command":command,"samples":samples,
    "median_elapsed_ms":statistics.median(s["elapsed_ms"] for s in samples),
    "median_peak_rss_bytes":statistics.median(s["peak_rss_bytes"] for s in samples) if all(s["peak_rss_bytes"] is not None for s in samples) else None}
report["cwd"] = str(args.cwd or Path.cwd())
source_digest = hashlib.sha256()
for path in sorted([Path("Cargo.toml"),Path("Cargo.lock"),*Path("crates").rglob("*.rs"),*Path("crates").rglob("*.xml"),*Path("crates").rglob("Cargo.toml")]):
    source_digest.update(str(path).encode());source_digest.update(path.read_bytes())
report["rust_source_sha256"] = source_digest.hexdigest() if args.implementation=="rust" else None
report["source_commit"] = "fef9860c816f796c26442c68218461cf375eaa38" if args.implementation=="electron" else report["commit"]
if args.input:
    digest = hashlib.sha256()
    files = sorted(p for p in args.input.rglob("*") if p.is_file() and p.suffix.lower() in [".svg",".csv"])
    for path in files:
        digest.update(str(path.relative_to(args.input)).encode());digest.update(path.read_bytes())
    report["dataset"] = {"sha256":digest.hexdigest(),"files":len(files),"svg_count":sum(p.suffix.lower()==".svg" for p in files),"concurrency":2,"prepared_csv":True}
if args.artifact:
    report["artifact_bytes"] = args.artifact.stat().st_size if args.artifact.is_file() else sum(p.stat().st_size for p in args.artifact.rglob("*") if p.is_file() and not p.is_symlink())
args.output.write_text(json.dumps(report,indent=2))
raise SystemExit(samples[-1]["exit_code"])
