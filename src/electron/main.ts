import path from "node:path";
import { fileURLToPath } from "node:url";

import { app, BrowserWindow, ipcMain } from "electron";

import type {
  CreateJobRequest,
  PrepareDbRequest,
  SearchResultsRequest,
  SearchStatusRequest,
  SearchSubmodelsRequest,
  SelectDirectoryRequest,
} from "../shared/api.js";
import { DesktopBackend } from "./backend.js";

const backend = new DesktopBackend();

function resolveRendererEntry(): { isDev: boolean; value: string } {
  const devServerUrl = process.env.VITE_DEV_SERVER_URL ?? "http://127.0.0.1:5173";
  const rendererPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../web-client/index.html");
  return app.isPackaged ? { isDev: false, value: rendererPath } : { isDev: true, value: devServerUrl };
}

async function createWindow(): Promise<void> {
  const window = new BrowserWindow({
    width: 1440,
    height: 960,
    minWidth: 1200,
    minHeight: 760,
    title: "NppToDocx",
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
      preload: path.resolve(path.dirname(fileURLToPath(import.meta.url)), "preload.js"),
    },
  });

  const rendererEntry = resolveRendererEntry();
  if (rendererEntry.isDev) {
    await window.loadURL(rendererEntry.value);
    window.webContents.openDevTools({ mode: "detach" });
    return;
  }

  await window.loadFile(rendererEntry.value);
}

function registerHandlers(): void {
  ipcMain.handle("app:get-config", () => backend.getConfig());
  ipcMain.handle("app:prepare-db", (_, payload) => backend.prepareDb(payload as PrepareDbRequest | undefined));
  ipcMain.handle("app:create-job", (_, payload) => backend.createJob(payload as CreateJobRequest | undefined));
  ipcMain.handle("app:list-jobs", () => backend.listJobs());
  ipcMain.handle("app:get-search-status", (_, payload) => backend.getSearchStatus(payload as SearchStatusRequest));
  ipcMain.handle("app:list-search-submodels", (_, payload) =>
    backend.listSearchSubmodels(payload as SearchSubmodelsRequest),
  );
  ipcMain.handle("app:search-by-submodel", (_, payload) => backend.searchBySubmodel(payload as SearchResultsRequest));
  ipcMain.handle("app:select-directory", (_, payload) => backend.selectDirectory(payload as SelectDirectoryRequest));
}

app.whenReady().then(async () => {
  registerHandlers();
  await createWindow();

  app.on("activate", async () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      await createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
