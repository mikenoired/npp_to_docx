"""Check every source marker, KKS, submodel and coordinate against the TS parser."""
import argparse
import json
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("expected", type=Path)
parser.add_argument("input", type=Path)
parser.add_argument("--binary", type=Path, default=Path("target/release/npp-convert"))
parser.add_argument("--report", type=Path, default=Path("output/marker-parity.json"))
args = parser.parse_args()
expected = json.loads(args.expected.read_text())
differences = []
for name, reference in sorted(expected.items()):
    process = subprocess.run([str(args.binary),"inspect","--svg",str(args.input / name)],capture_output=True,text=True)
    if process.returncode:
        differences.append({"file":name,"error":process.stderr})
        continue
    actual = json.loads(process.stdout)
    for key in ["viewWidth","viewHeight"]:
        if actual.get(key) != reference.get(key):
            differences.append({"file":name,"key":key,"expected":reference.get(key),"actual":actual.get(key)})
    if reference["markers"] != actual["markers"]:
        differences.append({"file":name,"expected_count":len(reference["markers"]),"actual_count":len(actual["markers"]),
            "examples":[(a,b) for a,b in zip(reference["markers"],actual["markers"]) if a != b][:4]})
result = {"pass":not differences,"files":len(expected),"differences":differences}
args.report.write_text(json.dumps(result,ensure_ascii=False,indent=2))
print(f"Checked {len(expected)} files. Differences: {len(differences)}. Report: {args.report}")
raise SystemExit(0 if not differences else 1)
