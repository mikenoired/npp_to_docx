import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { app, dialog } from "electron";

import { getDefaultConcurrency } from "../core/runtime.js";
import {
  findMissingSearchFiles,
  hasSearchIndex,
  InvalidSearchIndexError,
  listSubmodels,
  readSearchIndex,
  searchBySubmodel,
} from "../core/search/index.js";
import { ensureDatabaseCsvs } from "../core/services/pls-db.js";
import type {
  AppConfigResponse,
  CreateJobRequest,
  CreateJobResponse,
  JobsResponse,
  PrepareDbRequest,
  PrepareDbResponse,
  SearchResultsRequest,
  SearchResultsResponse,
  SearchStatusRequest,
  SearchStatusResponse,
  SearchSubmodelsRequest,
  SearchSubmodelsResponse,
  SelectDirectoryRequest,
} from "../shared/api.js";
import { JobManager } from "./jobs.js";

type AppSettings = {
  inputDir?: string;
  outputDir?: string;
};

type AppPaths = {
  workspaceDir: string;
  inputDir: string;
  outputDir: string;
  logsDir: string;
};

export class DesktopBackend {
  private readonly jobManager = new JobManager(async () => (await this.getAppPaths()).logsDir);

  async getConfig(): Promise<AppConfigResponse> {
    const appPaths = await this.getAppPaths();
    return {
      defaults: {
        workspaceDir: appPaths.workspaceDir,
        inputDir: appPaths.inputDir,
        outputDir: appPaths.outputDir,
        logsDir: appPaths.logsDir,
        concurrency: getDefaultConcurrency(),
      },
    };
  }

  async prepareDb(payload: PrepareDbRequest = {}): Promise<PrepareDbResponse> {
    const inputDir = payload.inputDir ?? (await this.getAppPaths()).inputDir;
    await this.persistSettings({ inputDir });
    const csvPaths = await ensureDatabaseCsvs(inputDir);
    return { inputDir, csvPaths };
  }

  async createJob(payload: CreateJobRequest = {}): Promise<CreateJobResponse> {
    if (payload.inputDir || payload.outputDir) {
      await this.persistSettings({
        inputDir: payload.inputDir,
        outputDir: payload.outputDir,
      });
    }

    const job = this.jobManager.createJob({
      inputDir: payload.inputDir ?? (await this.getAppPaths()).inputDir,
      outputDir: payload.outputDir ?? (await this.getAppPaths()).outputDir,
      concurrency: payload.concurrency ?? getDefaultConcurrency(),
      match: payload.match,
      limit: payload.limit,
    });

    return {
      id: job.id,
      status: job.status,
    };
  }

  listJobs(): JobsResponse {
    return { jobs: this.jobManager.listJobs() };
  }

  async getSearchStatus(payload: SearchStatusRequest = {}): Promise<SearchStatusResponse> {
    const appPaths = await this.getAppPaths();
    const inputDir = payload.inputDir ?? appPaths.inputDir;
    const outputDir = payload.outputDir ?? appPaths.outputDir;

    const missingFiles = await findMissingSearchFiles(inputDir, outputDir);
    const indexExists = await hasSearchIndex(outputDir);
    let recordsCount = 0;
    let submodelsCount = 0;
    let message = "Поиск станет доступен после успешной обработки и построения индекса";
    let indexValid = false;

    if (indexExists) {
      try {
        const index = await readSearchIndex(outputDir);
        recordsCount = index.records.length;
        submodelsCount = listSubmodels(index).length;
        message = "Поиск готов";
        indexValid = true;
      } catch (error) {
        if (error instanceof InvalidSearchIndexError) {
          message = error.message;
        } else {
          throw error;
        }
      }
    }

    return {
      ready: missingFiles.length === 0 && indexExists && indexValid,
      indexExists,
      recordsCount,
      submodelsCount,
      missingFiles,
      message:
        missingFiles.length > 0 ? "Поиск станет доступен после успешной обработки и построения индекса" : message,
    };
  }

  async listSearchSubmodels(payload: SearchSubmodelsRequest = {}): Promise<SearchSubmodelsResponse> {
    const outputDir = payload.outputDir ?? (await this.getAppPaths()).outputDir;
    const index = await this.readValidSearchIndex(outputDir);
    return {
      items: listSubmodels(index, payload.query),
    };
  }

  async searchBySubmodel(payload: SearchResultsRequest): Promise<SearchResultsResponse> {
    const outputDir = payload.outputDir ?? (await this.getAppPaths()).outputDir;
    const index = await this.readValidSearchIndex(outputDir);
    return {
      submodel: payload.submodel,
      items: searchBySubmodel(index, payload.submodel),
    };
  }

  async selectDirectory(payload: SelectDirectoryRequest = {}): Promise<string | undefined> {
    const result = await dialog.showOpenDialog({
      title: payload.title ?? "Выберите директорию",
      defaultPath: payload.defaultPath,
      properties: ["openDirectory", "createDirectory"],
    });

    if (result.canceled) {
      return undefined;
    }

    return result.filePaths[0];
  }

  private async getAppPaths(): Promise<AppPaths> {
    const workspaceDir = path.join(app.getPath("documents"), "npp_to_docx");
    const settings = await this.loadSettings();
    const inputDir = settings.inputDir ?? path.join(workspaceDir, "input");
    const outputDir = settings.outputDir ?? path.join(workspaceDir, "output");
    const logsDir = path.join(workspaceDir, "logs");

    await Promise.all([
      mkdir(workspaceDir, { recursive: true }),
      mkdir(inputDir, { recursive: true }),
      mkdir(outputDir, { recursive: true }),
      mkdir(logsDir, { recursive: true }),
    ]);

    return {
      workspaceDir,
      inputDir,
      outputDir,
      logsDir,
    };
  }

  private async loadSettings(): Promise<AppSettings> {
    const settingsPath = this.getSettingsPath();
    try {
      const raw = await readFile(settingsPath, "utf8");
      return JSON.parse(raw) as AppSettings;
    } catch {
      return {};
    }
  }

  private async persistSettings(patch: AppSettings): Promise<void> {
    const settings = await this.loadSettings();
    const nextSettings: AppSettings = {
      inputDir: patch.inputDir ?? settings.inputDir,
      outputDir: patch.outputDir ?? settings.outputDir,
    };

    await mkdir(path.dirname(this.getSettingsPath()), { recursive: true });
    await writeFile(this.getSettingsPath(), `${JSON.stringify(nextSettings, null, 2)}\n`, "utf8");
  }

  private getSettingsPath(): string {
    return path.join(app.getPath("userData"), "settings.json");
  }

  private async readValidSearchIndex(outputDir: string) {
    try {
      return await readSearchIndex(outputDir);
    } catch (error) {
      if (error instanceof InvalidSearchIndexError) {
        throw new Error("Поисковый индекс поврежден. Запустите обработку заново, чтобы пересоздать search-index.json.");
      }
      throw error;
    }
  }
}
