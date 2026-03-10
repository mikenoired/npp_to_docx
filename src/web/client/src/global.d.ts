import type { AppApi } from "../../../shared/api";

declare global {
  interface Window {
    nppApi: AppApi;
  }
}
