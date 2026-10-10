// Development benchmark only. The migrated desktop application has no Node runtime.
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { writeFile } from 'node:fs/promises';

const [baseline, input, output, resultPath] = process.argv.slice(2);
if (!baseline || !input || !output || !resultPath) {
  throw new Error('Usage: node scripts/electron_run.mjs BASELINE INPUT OUTPUT RESULT_JSON');
}
const { processBatch } = await import(pathToFileURL(path.resolve(baseline, 'build/core/use-cases/process-batch.js')));
const start = performance.now();
const result = await processBatch({ inputDir: input, outputDir: output, concurrency: 2 }, { log: console.error });
await writeFile(resultPath, JSON.stringify({ ...result, elapsed_ms: performance.now() - start }, null, 2));
console.log(JSON.stringify(result));
if (result.failed) process.exitCode = 1;
