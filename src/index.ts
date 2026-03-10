export { runCli } from "./cli/run-cli";
export type { BatchProgress, BatchResult, LoggerLike, ParsedSvg, ProcessBatchOptions } from "./core/contracts";
export { FileLogger } from "./core/logger/file-logger";
export { getDefaultConcurrency } from "./core/runtime";
export { ensureDatabaseCsvs, loadDescriptionIndex, lookupDescription } from "./core/services/pls-db";
export { processBatch } from "./core/use-cases/process-batch";

if (import.meta.main) {
  await import("./cli/run-cli").then(({ runCli }) => runCli());
}
