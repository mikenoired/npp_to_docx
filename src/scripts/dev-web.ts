import { type ChildProcess, spawn } from "node:child_process";

const bunPath = process.execPath;
const rootDir = process.cwd();
const serverPort = process.env.DEV_WEB_SERVER_PORT ?? "3300";
const uiPort = process.env.DEV_WEB_UI_PORT ?? "5173";

function startProcess(args: string[], env: NodeJS.ProcessEnv): ChildProcess {
  return spawn(bunPath, args, {
    cwd: rootDir,
    stdio: "inherit",
    env,
  });
}

const server = startProcess(["--hot", "src/web/server/main.ts"], {
  ...process.env,
  PORT: serverPort,
});

const ui = startProcess(["run", "dev:web:ui", "--", "--port", uiPort], {
  ...process.env,
  VITE_API_TARGET: process.env.VITE_API_TARGET ?? `http://127.0.0.1:${serverPort}`,
});

let shuttingDown = false;

function shutdown(exitCode = 0): void {
  if (shuttingDown) {
    return;
  }
  shuttingDown = true;
  server.kill("SIGTERM");
  ui.kill("SIGTERM");
  setTimeout(() => process.exit(exitCode), 200);
}

server.on("exit", (code) => {
  if (!shuttingDown) {
    shutdown(code ?? 1);
  }
});

ui.on("exit", (code) => {
  if (!shuttingDown) {
    shutdown(code ?? 1);
  }
});

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));

console.log(`dev-web started: ui=http://127.0.0.1:${uiPort} api=http://127.0.0.1:${serverPort}`);
