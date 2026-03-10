import { contextBridge, ipcRenderer } from "electron";

import type {
  AppApi,
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

function invoke<T>(channel: string, payload?: unknown): Promise<T> {
  return ipcRenderer.invoke(channel, payload) as Promise<T>;
}

const api: AppApi = {
  getConfig: () => invoke<AppConfigResponse>("app:get-config"),
  prepareDb: (payload: PrepareDbRequest) => invoke<PrepareDbResponse>("app:prepare-db", payload),
  createJob: (payload: CreateJobRequest) => invoke<CreateJobResponse>("app:create-job", payload),
  listJobs: () => invoke<JobsResponse>("app:list-jobs"),
  getSearchStatus: (payload: SearchStatusRequest) => invoke<SearchStatusResponse>("app:get-search-status", payload),
  listSearchSubmodels: (payload: SearchSubmodelsRequest) =>
    invoke<SearchSubmodelsResponse>("app:list-search-submodels", payload),
  searchBySubmodel: (payload: SearchResultsRequest) => invoke<SearchResultsResponse>("app:search-by-submodel", payload),
  selectDirectory: (payload: SelectDirectoryRequest) => invoke<string | undefined>("app:select-directory", payload),
};

contextBridge.exposeInMainWorld("nppApi", api);
