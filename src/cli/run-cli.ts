import path from "node:path";
import { fileURLToPath } from "node:url";

import { FileLogger } from "../core/logger/file-logger";
import { processBatch } from "../core/use-cases/process-batch";
import { parseArgs } from "./args";

function formatDateTimeForFileName(value: Date): string {
  const pad = (number: number) => String(number).padStart(2, "0");
  const year = value.getFullYear();
  const month = pad(value.getMonth() + 1);
  const day = pad(value.getDate());
  const hour = pad(value.getHours());
  const minute = pad(value.getMinutes());
  const second = pad(value.getSeconds());
  return `${year}-${month}-${day}_${hour}-${minute}-${second}`;
}

export async function runCli(argv = process.argv.slice(2)): Promise<void> {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const projectRoot = path.resolve(scriptDir, "../..");
  const logFilePath = path.join(projectRoot, `${formatDateTimeForFileName(new Date())}.log`);
  const logger = new FileLogger(logFilePath);
  const options = parseArgs(argv);
  let lastProgressLength = 0;

  logger.log("Process started");

  try {
    const result = await processBatch(options, logger, {
      onProgress: (progress) => {
        const paddedLine = progress.line.padEnd(lastProgressLength, " ");
        process.stdout.write(`\r${paddedLine}`);
        lastProgressLength = paddedLine.length;
      },
    });

    process.stdout.write("\n");
    logger.log("Process finished");
    console.log(`${result.summary}. Лог: ${path.basename(logFilePath)}`);
  } finally {
    await logger.close();
  }
}
