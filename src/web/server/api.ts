import { getDefaultConcurrency } from "../../core/runtime";
import { ensureDatabaseCsvs } from "../../core/services/pls-db";
import type {
  AppConfigResponse,
  CreateJobRequest,
  CreateJobResponse,
  JobLogsResponse,
  JobsResponse,
  PrepareDbRequest,
  PrepareDbResponse,
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
