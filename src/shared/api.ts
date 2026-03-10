import type { BatchProgress, BatchResult, ProcessBatchOptions } from "../core/contracts.js";

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
    workspaceDir: string;
    inputDir: string;
    outputDir: string;
    logsDir: string;
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

export type SearchStatusRequest = {
  inputDir?: string;
  outputDir?: string;
};

export type SearchSubmodelsRequest = {
  outputDir?: string;
  query?: string;
};

export type SearchResultsRequest = {
  outputDir?: string;
  submodel: string;
};

export type SelectDirectoryRequest = {
  title?: string;
  defaultPath?: string;
};

export type AppApi = {
  getConfig(): Promise<AppConfigResponse>;
  prepareDb(payload: PrepareDbRequest): Promise<PrepareDbResponse>;
  createJob(payload: CreateJobRequest): Promise<CreateJobResponse>;
  listJobs(): Promise<JobsResponse>;
  getSearchStatus(payload: SearchStatusRequest): Promise<SearchStatusResponse>;
  listSearchSubmodels(payload: SearchSubmodelsRequest): Promise<SearchSubmodelsResponse>;
  searchBySubmodel(payload: SearchResultsRequest): Promise<SearchResultsResponse>;
  selectDirectory(payload: SelectDirectoryRequest): Promise<string | undefined>;
};
