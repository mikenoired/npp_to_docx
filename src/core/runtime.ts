import os from "node:os";

export function getDefaultConcurrency(): number {
  return Math.max(1, Math.min(6, Math.floor(os.cpus().length / 2) || 1));
}
