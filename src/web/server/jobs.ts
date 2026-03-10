import { randomUUID } from "node:crypto";

import type { ProcessBatchOptions } from "../../core/contracts";
import { getDefaultConcurrency } from "../../core/runtime";
import { processBatch } from "../../core/use-cases/process-batch";
import type { CreateJobRequest, JobDto, JobStatus } from "../../shared/api";

type JobState = JobDto;

const jobs = new Map<string, JobState>();

function normalizeJob(job: JobState): JobDto {
  return {
    id: job.id,
    status: job.status,
    options: job.options,
    createdAt: job.createdAt,
    startedAt: job.startedAt,
    finishedAt: job.finishedAt,
    progress: job.progress,
    result: job.result,
    error: job.error,
    logs: job.logs,
  };
}

async function startJob(job: JobState): Promise<void> {
  job.status = "running";
  job.startedAt = new Date().toISOString();

  const logger = {
    log(message: string) {
      job.logs.push(message);
      if (job.logs.length > 500) {
        job.logs.shift();
      }
    },
  };

  try {
    const result = await processBatch(job.options, logger, {
      onProgress(progress) {
        job.progress = progress;
      },
    });

    job.result = result;
    job.status = "done";
    job.finishedAt = new Date().toISOString();
  } catch (error) {
    job.error = error instanceof Error ? error.message : String(error);
    job.status = "failed";
    job.finishedAt = new Date().toISOString();
  }
}

function normalizeOptions(payload: CreateJobRequest): ProcessBatchOptions {
  return {
    inputDir: payload.inputDir ?? "input",
    outputDir: payload.outputDir ?? "output",
    concurrency: payload.concurrency ?? getDefaultConcurrency(),
    match: payload.match,
    limit: payload.limit,
  };
}

export function createJob(payload: CreateJobRequest): JobDto {
  const job: JobState = {
    id: randomUUID(),
    status: "queued" satisfies JobStatus,
    options: normalizeOptions(payload),
    createdAt: new Date().toISOString(),
    logs: [],
  };

  jobs.set(job.id, job);
  void startJob(job);
  return normalizeJob(job);
}

export function listJobs(): JobDto[] {
  return [...jobs.values()].map(normalizeJob);
}

export function getJob(id: string): JobDto | undefined {
  const job = jobs.get(id);
  return job ? normalizeJob(job) : undefined;
}
