import { randomUUID } from "node:crypto";
import { mkdir } from "node:fs/promises";
import path from "node:path";

import type { ProcessBatchOptions } from "../core/contracts.js";
import { FileLogger } from "../core/logger/file-logger.js";
import { getDefaultConcurrency } from "../core/runtime.js";
import { processBatch } from "../core/use-cases/process-batch.js";
import type { CreateJobRequest, JobDto, JobStatus } from "../shared/api.js";

type JobState = JobDto;

function formatDateTimeForFileName(value: Date): string {
  const pad = (number: number) => String(number).padStart(2, "0");
  return [
    value.getFullYear(),
    pad(value.getMonth() + 1),
    pad(value.getDate()),
    "_",
    pad(value.getHours()),
    "-",
    pad(value.getMinutes()),
    "-",
    pad(value.getSeconds()),
  ].join("");
}

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
    logs: [...job.logs],
  };
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

export class JobManager {
  private readonly jobs = new Map<string, JobState>();

  constructor(private readonly getLogsDir: () => Promise<string>) {}

  listJobs(): JobDto[] {
    return [...this.jobs.values()]
      .map(normalizeJob)
      .sort((left, right) => right.createdAt.localeCompare(left.createdAt));
  }

  getJob(id: string): JobDto | undefined {
    const job = this.jobs.get(id);
    return job ? normalizeJob(job) : undefined;
  }

  createJob(payload: CreateJobRequest): JobDto {
    const job: JobState = {
      id: randomUUID(),
      status: "queued" satisfies JobStatus,
      options: normalizeOptions(payload),
      createdAt: new Date().toISOString(),
      logs: [],
    };

    this.jobs.set(job.id, job);
    void this.startJob(job);
    return normalizeJob(job);
  }

  private async startJob(job: JobState): Promise<void> {
    job.status = "running";
    job.startedAt = new Date().toISOString();

    const logsDir = await this.getLogsDir();
    await mkdir(logsDir, { recursive: true });

    const logFilePath = path.join(logsDir, `${formatDateTimeForFileName(new Date())}_${job.id.slice(0, 8)}.log`);
    const fileLogger = new FileLogger(logFilePath);
    const logger = {
      log: (message: string) => {
        job.logs.push(message);
        if (job.logs.length > 500) {
          job.logs.shift();
        }
        fileLogger.log(message);
      },
    };

    try {
      const result = await processBatch(job.options, logger, {
        onProgress(progress) {
          job.progress = progress;
        },
      });

      job.result = {
        ...result,
        logFilePath,
      };
      job.status = "done";
      job.finishedAt = new Date().toISOString();
    } catch (error) {
      job.error = error instanceof Error ? error.message : String(error);
      job.status = "failed";
      job.finishedAt = new Date().toISOString();
    } finally {
      await fileLogger.close();
    }
  }
}
