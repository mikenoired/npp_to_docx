"""Package the release executable using the host platform's native layout."""
import argparse
import plistlib
import shutil
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--binary",type=Path,default=Path("target/release/npp-to-docx.exe" if sys.platform == "win32" else "target/release/npp-to-docx"))
parser.add_argument("--output",type=Path,default=Path("dist/rust"))
args = parser.parse_args()
args.output.mkdir(parents=True,exist_ok=True)
if not args.binary.is_file():
    raise SystemExit("Run cargo build --release --locked first")
if sys.platform == "darwin":
    app = args.output / "NppToDocx.app"
    executable = app / "Contents/MacOS/npp-to-docx"
    executable.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(args.binary,executable)
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps({
        "CFBundleIdentifier":"local.npp-to-docx.desktop", "CFBundleName":"NppToDocx",
        "CFBundleExecutable":"npp-to-docx", "CFBundlePackageType":"APPL",
        "CFBundleShortVersionString":"2.0.0", "CFBundleVersion":"2.0.0",
        "NSHighResolutionCapable":True, "LSMinimumSystemVersion":"12.0",
    }))
    subprocess.run(["codesign","--force","--deep","--sign","-",str(app)],check=True)
    destination = args.output / "NppToDocx-macos.zip"
    subprocess.run(["ditto","-c","-k","--sequesterRsrc","--keepParent",str(app),str(destination)],check=True)
elif sys.platform == "win32":
    destination = args.output / "NppToDocx-windows.zip"
    with zipfile.ZipFile(destination,"w",compression=zipfile.ZIP_DEFLATED) as archive:
        archive.write(args.binary,"NppToDocx/npp-to-docx.exe")
        archive.write("README.md","NppToDocx/README.md")
else:
    destination = args.output / "NppToDocx-linux.tar.gz"
    with tarfile.open(destination,"w:gz") as archive:
        archive.add(args.binary,arcname="NppToDocx/npp-to-docx")
        archive.add("README.md",arcname="NppToDocx/README.md")
print(destination)
