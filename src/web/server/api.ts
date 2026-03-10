import { getDefaultConcurrency } from "../../core/runtime";
import {
  findMissingSearchFiles,
  hasSearchIndex,
  listSubmodels,
  readSearchIndex,
  searchBySubmodel,
} from "../../core/search/index";
import { ensureDatabaseCsvs } from "../../core/services/pls-db";
import type {
  AppConfigResponse,
  CreateJobRequest,
  CreateJobResponse,
  JobLogsResponse,
  JobsResponse,
  PrepareDbRequest,
  PrepareDbResponse,
  SearchResultsResponse,
  SearchStatusResponse,
  SearchSubmodelsResponse,
} from "../../shared/api";
import { createJob, getJob, listJobs } from "./jobs";

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data, null, 2), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
    },
  });
}

export async function handleApi(request: Request): Promise<Response> {
  const url = new URL(request.url);

  if (request.method === "GET" && url.pathname === "/api/health") {
    return json({ ok: true });
  }

  if (request.method === "GET" && url.pathname === "/api/config") {
    const response: AppConfigResponse = {
      defaults: {
        inputDir: "input",
        outputDir: "output",
        concurrency: getDefaultConcurrency(),
      },
    };
    return json(response);
  }

  if (request.method === "GET" && url.pathname === "/api/search/status") {
    const inputDir = url.searchParams.get("inputDir") ?? "input";
    const outputDir = url.searchParams.get("outputDir") ?? "output";
    const missingFiles = await findMissingSearchFiles(inputDir, outputDir);
    const indexExists = await hasSearchIndex(outputDir);
    let recordsCount = 0;
    let submodelsCount = 0;
    if (indexExists) {
      const index = await readSearchIndex(outputDir);
      recordsCount = index.records.length;
      submodelsCount = listSubmodels(index).length;
    }
    const response: SearchStatusResponse = {
      ready: missingFiles.length === 0 && indexExists,
      indexExists,
      recordsCount,
      submodelsCount,
      missingFiles,
      message:
        missingFiles.length === 0 && indexExists
          ? "Поиск готов"
          : "Поиск станет доступен после успешной обработки и построения индекса",
    };
    return json(response);
  }

  if (request.method === "GET" && url.pathname === "/api/search/submodels") {
    const outputDir = url.searchParams.get("outputDir") ?? "output";
    const query = url.searchParams.get("q") ?? undefined;
    const index = await readSearchIndex(outputDir);
    const response: SearchSubmodelsResponse = {
      items: listSubmodels(index, query),
    };
    return json(response);
  }

  if (request.method === "GET" && url.pathname === "/api/search/results") {
    const outputDir = url.searchParams.get("outputDir") ?? "output";
    const submodel = url.searchParams.get("submodel");
    if (!submodel) {
      return json({ error: "submodel is required" }, 400);
    }
    const index = await readSearchIndex(outputDir);
    const response: SearchResultsResponse = {
      submodel,
      items: searchBySubmodel(index, submodel),
    };
    return json(response);
  }

  if (request.method === "POST" && url.pathname === "/api/db/prepare") {
    const payload = (await request.json()) as PrepareDbRequest;
    const inputDir = payload.inputDir ?? "input";
    const csvPaths = await ensureDatabaseCsvs(inputDir);
    const response: PrepareDbResponse = { inputDir, csvPaths };
    return json(response);
  }

  if (request.method === "POST" && url.pathname === "/api/jobs") {
    const payload = (await request.json()) as CreateJobRequest;
    const job = createJob(payload);
    const response: CreateJobResponse = { id: job.id, status: job.status };
    return json(response, 202);
  }

  if (request.method === "GET" && url.pathname === "/api/jobs") {
    const response: JobsResponse = { jobs: listJobs() };
    return json(response);
  }

  const jobMatch = url.pathname.match(/^\/api\/jobs\/([^/]+)$/);
  if (request.method === "GET" && jobMatch) {
    const job = getJob(jobMatch[1]);
    if (!job) {
      return json({ error: "Job not found" }, 404);
    }
    return json(job);
  }

  const logMatch = url.pathname.match(/^\/api\/jobs\/([^/]+)\/log$/);
  if (request.method === "GET" && logMatch) {
    const job = getJob(logMatch[1]);
    if (!job) {
      return json({ error: "Job not found" }, 404);
    }
    const response: JobLogsResponse = { id: job.id, logs: job.logs };
    return json(response);
  }

  return json({ error: "Not found" }, 404);
}
