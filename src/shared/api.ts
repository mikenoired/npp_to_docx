import type { BatchProgress, BatchResult, ProcessBatchOptions } from "../core/contracts";

export type JobStatus = "queued" | "running" | "done" | "failed";

export type JobDto = {
  id: string;
  status: JobStatus;
  options: ProcessBatchOptions;
  createdAt: string;
  startedAt?: string;
  finishedAt?: string;
  progress?: BatchProgress;
  result?: BatchResult;
  error?: string;
  logs: string[];
};

export type AppConfigResponse = {
  defaults: {
    inputDir: string;
    outputDir: string;
    concurrency: number;
  };
};

export type PrepareDbRequest = {
  inputDir?: string;
};

export type PrepareDbResponse = {
  inputDir: string;
  csvPaths: string[];
};

export type CreateJobRequest = Partial<ProcessBatchOptions>;

export type CreateJobResponse = {
  id: string;
  status: JobStatus;
};

export type JobsResponse = {
  jobs: JobDto[];
};

export type JobLogsResponse = {
  id: string;
  logs: string[];
};

export type SearchSubmodelOption = {
  submodel: string;
  count: number;
};

export type SearchResultItem = {
  frameName: string;
  markerIndex: number;
  submodel: string;
  kks?: string;
  title: string;
  description?: string;
};

export type SearchStatusResponse = {
  ready: boolean;
  indexExists: boolean;
  recordsCount: number;
  submodelsCount: number;
  missingFiles: string[];
  message: string;
};

export type SearchSubmodelsResponse = {
  items: SearchSubmodelOption[];
};

export type SearchResultsResponse = {
  submodel: string;
  items: SearchResultItem[];
};
