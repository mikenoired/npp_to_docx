"""Restore the measured Electron commit in ignored output for reproducible reruns."""
import io
import shutil
import subprocess
import zipfile
from pathlib import Path

COMMIT = "fef9860c816f796c26442c68218461cf375eaa38"
destination = Path("output/electron-baseline")
destination.mkdir(parents=True,exist_ok=True)
archive = subprocess.check_output(["git","archive","--format=zip",COMMIT])
with zipfile.ZipFile(io.BytesIO(archive)) as source:
    source.extractall(destination)
shutil.copy2("benchmarks/electron-package-lock.json",destination / "package-lock.json")
subprocess.run(["npm","ci","--no-audit","--no-fund"],cwd=destination,check=True)
subprocess.run(["npm","run","build"],cwd=destination,check=True)
print(destination)
