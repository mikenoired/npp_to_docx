import type { ProcessBatchOptions } from "../core/contracts";
import { getDefaultConcurrency } from "../core/runtime";

function printHelp(): void {
  console.log(`Использование: bun run src/cli/main.ts [options]

Опции:
  --input <dir>         Путь до корневой папки input с dmp/csv и подпапкой svg (по умолчанию: input)
  --output <dir>        Путь до папки, где будут готовые DOCX-файлы (по умолчанию: output)
  --concurrency <n>     Кол-во паралельных обработок (по умолчанию: ядра процессора / 2, макс. 6)
  --match <text>        Обработать только те svg, которые содержать некоторый текст
  --limit <n>           Обработать только N-ое кол-во файлов с начала
  --help                Показать справочник`);
}

export function parseArgs(argv: string[]): ProcessBatchOptions {
  const options: ProcessBatchOptions = {
    inputDir: "input",
    outputDir: "output",
    concurrency: getDefaultConcurrency(),
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help") {
      printHelp();
      process.exit(0);
    }
    if (arg === "--input" && argv[index + 1]) {
      options.inputDir = argv[index + 1];
      index += 1;
      continue;
    }
    if (arg === "--output" && argv[index + 1]) {
      options.outputDir = argv[index + 1];
      index += 1;
      continue;
    }
    if (arg === "--concurrency" && argv[index + 1]) {
      const parsed = Number.parseInt(argv[index + 1], 10);
      if (Number.isFinite(parsed) && parsed > 0) {
        options.concurrency = parsed;
      }
      index += 1;
      continue;
    }
    if (arg === "--match" && argv[index + 1]) {
      options.match = argv[index + 1];
      index += 1;
      continue;
    }
    if (arg === "--limit" && argv[index + 1]) {
      const parsed = Number.parseInt(argv[index + 1], 10);
      if (Number.isFinite(parsed) && parsed > 0) {
        options.limit = parsed;
      }
      index += 1;
      continue;
    }
    throw new Error(`Неизвестный аргумент: ${arg}`);
  }

  return options;
}
