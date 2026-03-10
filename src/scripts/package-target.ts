import { spawnSync } from "node:child_process";
import { chmod, cp, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDir, "../..");
const distDir = path.join(projectRoot, "dist");
const targetDir = path.join(projectRoot, "target");
const appName = "NppToDocx.app";
const appDir = path.join(targetDir, appName);
const contentsDir = path.join(appDir, "Contents");
const macOsDir = path.join(contentsDir, "MacOS");
const resourcesDir = path.join(contentsDir, "Resources");
const launcherPath = path.join(macOsDir, "NppToDocx");
const bundledWebPath = path.join(resourcesDir, "npp-web");
const bundledCliPath = path.join(targetDir, "npp-cli");
const infoPlistPath = path.join(contentsDir, "Info.plist");
const readmePath = path.join(targetDir, "README.txt");

function runBun(args: string[]): void {
  const result = spawnSync(process.execPath, args, {
    cwd: projectRoot,
    stdio: "inherit",
    env: process.env,
  });

  if (result.status !== 0) {
    throw new Error(`Command failed: bun ${args.join(" ")}`);
  }
}

function buildInfoPlist(): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>ru</string>
  <key>CFBundleExecutable</key>
  <string>NppToDocx</string>
  <key>CFBundleIdentifier</key>
  <string>local.npp-to-docx.web</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>NppToDocx</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
</dict>
</plist>
`;
}

function buildLauncherScript(): string {
  return `#!/bin/zsh
set -euo pipefail

APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$(cd "$APP_DIR/../.." && pwd)"
RES_DIR="$APP_DIR/Resources"
PORT="\${NPP_TO_DOCX_PORT:-3210}"
PID_FILE="$TARGET_DIR/.npp-web.pid"
LOG_DIR="$TARGET_DIR/logs"
LOG_FILE="$LOG_DIR/npp-web.log"

mkdir -p "$LOG_DIR" "$TARGET_DIR/output" "$TARGET_DIR/input/svg"
cd "$TARGET_DIR"

if [[ -f "$PID_FILE" ]]; then
  PID="$(cat "$PID_FILE")"
  if kill -0 "$PID" >/dev/null 2>&1; then
    open "http://127.0.0.1:$PORT"
    exit 0
  fi
  rm -f "$PID_FILE"
fi

env PORT="$PORT" /usr/bin/nohup "$RES_DIR/npp-web" >> "$LOG_FILE" 2>&1 &
PID=$!
echo "$PID" > "$PID_FILE"

for _ in {1..80}; do
  if /usr/bin/curl -sf "http://127.0.0.1:$PORT/api/health" >/dev/null 2>&1; then
    open "http://127.0.0.1:$PORT"
    exit 0
  fi
  sleep 0.25
done

open "http://127.0.0.1:$PORT"
`;
}

function buildTargetReadme(): string {
  return `NppToDocx target package

Что запускать:
- двойной клик по ${appName}

Что лежит рядом:
- input/  : входные dmp/csv/svg
- output/ : готовые docx и search-index.json
- logs/   : лог web-сервера
- npp-cli : CLI бинарник для терминала

Если нужно сменить порт:
- запустите app с переменной NPP_TO_DOCX_PORT
`;
}

async function ensureTargetLayout(): Promise<void> {
  await rm(targetDir, { recursive: true, force: true });
  await mkdir(macOsDir, { recursive: true });
  await mkdir(resourcesDir, { recursive: true });
  await mkdir(path.join(targetDir, "output"), { recursive: true });
  await mkdir(path.join(targetDir, "logs"), { recursive: true });
  await mkdir(path.join(targetDir, "input"), { recursive: true });
}

async function copyRuntimeData(): Promise<void> {
  const inputSource = path.join(projectRoot, "input");
  await cp(inputSource, path.join(targetDir, "input"), {
    recursive: true,
    force: true,
  });
}

async function main(): Promise<void> {
  runBun(["run", "build:bins"]);
  await ensureTargetLayout();
  await copyRuntimeData();

  await cp(path.join(distDir, "npp-web"), bundledWebPath, { force: true });
  await cp(path.join(distDir, "npp-cli"), bundledCliPath, { force: true });
  await writeFile(infoPlistPath, buildInfoPlist(), "utf8");
  await writeFile(launcherPath, buildLauncherScript(), "utf8");
  await writeFile(readmePath, buildTargetReadme(), "utf8");
  await chmod(launcherPath, 0o755);
  await chmod(bundledWebPath, 0o755);
  await chmod(bundledCliPath, 0o755);

  console.log(`Target package created: ${targetDir}`);
  console.log(`Launch by double click: ${path.join(targetDir, appName)}`);
}

await main();
