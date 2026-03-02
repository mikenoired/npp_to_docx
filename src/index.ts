import { createWriteStream, type WriteStream } from "node:fs";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { AlignmentType, Document, ImageRun, Packer, Paragraph, TextRun } from "docx";
import iconv from "iconv-lite";
import sax from "sax";
import sharp from "sharp";

type CliOptions = {
  inputDir: string;
  outputDir: string;
  concurrency: number;
  match?: string;
  limit?: number;
};

type Point = {
  x: number;
  y: number;
};

type RawMarker = {
  index: number;
  title: string;
  tag: string;
  point?: Point;
};

type RenderedMarker = {
  index: number;
  title: string;
  x: number;
  y: number;
};

type ParsedSvg = {
  markers: RawMarker[];
  viewWidth?: number;
  viewHeight?: number;
};

type ElementCtx = {
  name: string;
  attrs: Record<string, string>;
  titleText?: string;
  textContent?: string;
  firstChildPoint?: Point;
};

class FileLogger {
  private stream: WriteStream;

  constructor(public readonly filePath: string) {
    this.stream = createWriteStream(filePath, { flags: "a", encoding: "utf8" });
  }

  log(message: string): void {
    this.stream.write(`[${new Date().toISOString()}] ${message}\n`);
  }

  async close(): Promise<void> {
    await new Promise<void>((resolve, reject) => {
      this.stream.once("error", reject);
      this.stream.end(() => resolve());
    });
  }
}

function printHelp(): void {
  console.log(`Использование: bun run src/index.ts [options]

Опции:
  --input <dir>         Путь до папки, содержащий svg (по умолчанию: input)
  --output <dir>        Путь до папки, где будут готовые DOCX-файлы (по умолчанию: output)
  --concurrency <n>     Кол-во паралельных обработок (по умолчанию: ядра процессора / 2, макс. 6)
  --match <text>        Обработать только те svg, которые содержать некоторый текст
  --limit <n>           Обработать только N-ое кол-во файлов с начала
  --help                Показать справочник`);
}

function formatDateTimeForFileName(value: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  const year = value.getFullYear();
  const month = pad(value.getMonth() + 1);
  const day = pad(value.getDate());
  const hour = pad(value.getHours());
  const minute = pad(value.getMinutes());
  const second = pad(value.getSeconds());
  return `${year}-${month}-${day}_${hour}-${minute}-${second}`;
}

function renderProgressBar(done: number, total: number, success: number, failed: number, startTs: number): string {
  const width = 28;
  const ratio = total === 0 ? 0 : done / total;
  const filled = Math.round(ratio * width);
  const bar = `${"=".repeat(filled)}${"-".repeat(Math.max(0, width - filled))}`;
  const elapsedSec = (Date.now() - startTs) / 1000;
  return `[${bar}] ${done}/${total} ok:${success} fail:${failed} ${elapsedSec.toFixed(1)}s`;
}

function parseArgs(argv: string[]): CliOptions {
  const opts: CliOptions = {
    inputDir: "input",
    outputDir: "output",
    concurrency: Math.max(1, Math.min(6, Math.floor(os.cpus().length / 2) || 1)),
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help") {
      printHelp();
      process.exit(0);
    }
    if (arg === "--input" && argv[i + 1]) {
      opts.inputDir = argv[i + 1];
      i += 1;
      continue;
    }
    if (arg === "--output" && argv[i + 1]) {
      opts.outputDir = argv[i + 1];
      i += 1;
      continue;
    }
    if (arg === "--concurrency" && argv[i + 1]) {
      const parsed = Number.parseInt(argv[i + 1], 10);
      if (Number.isFinite(parsed) && parsed > 0) {
        opts.concurrency = parsed;
      }
      i += 1;
      continue;
    }
    if (arg === "--match" && argv[i + 1]) {
      opts.match = argv[i + 1];
      i += 1;
      continue;
    }
    if (arg === "--limit" && argv[i + 1]) {
      const parsed = Number.parseInt(argv[i + 1], 10);
      if (Number.isFinite(parsed) && parsed > 0) {
        opts.limit = parsed;
      }
      i += 1;
      continue;
    }
    throw new Error(`Неизвестный аргумент: ${arg}`);
  }

  return opts;
}

function normalizeEncoding(label: string): string {
  const lowered = label.trim().toLowerCase();
  if (lowered === "utf8") {
    return "utf-8";
  }
  if (lowered === "windows1251") {
    return "windows-1251";
  }
  if (lowered === "cp1251") {
    return "windows-1251";
  }
  return lowered;
}

function decodeSvgBuffer(buffer: Buffer): { content: string; encoding: string } {
  const prolog = buffer.subarray(0, 512).toString("latin1");
  const encodingMatch = prolog.match(/encoding\s*=\s*["']([^"']+)["']/i);
  const requested = normalizeEncoding(encodingMatch?.[1] ?? "utf-8");
  const encoding = iconv.encodingExists(requested) ? requested : "utf-8";
  const content = iconv.decode(buffer, encoding);
  return { content, encoding };
}

function toUtf8Xml(svgContent: string): string {
  if (/^<\?xml/i.test(svgContent)) {
    return svgContent.replace(/(<\?xml[^>]*encoding\s*=\s*["'])[^"']+(["'][^>]*\?>)/i, "$1UTF-8$2");
  }
  return svgContent;
}

function parseNumeric(value: string | undefined): number | undefined {
  if (!value) {
    return undefined;
  }
  const match = value.match(/-?\d+(\.\d+)?/);
  if (!match) {
    return undefined;
  }
  const parsed = Number.parseFloat(match[0]);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function parsePoints(points: string | undefined): Point | undefined {
  if (!points) {
    return undefined;
  }
  const pairs = points.trim().split(/\s+/);
  for (const pair of pairs) {
    const [xRaw, yRaw] = pair.split(",");
    const x = parseNumeric(xRaw);
    const y = parseNumeric(yRaw);
    if (x !== undefined && y !== undefined) {
      return { x, y };
    }
  }
  return undefined;
}

function decodeXmlEntities(input: string): string {
  return input.replace(/&(#x?[0-9a-fA-F]+|[a-zA-Z]+);/g, (_, entity: string) => {
    const lowered = entity.toLowerCase();
    if (lowered === "amp") return "&";
    if (lowered === "lt") return "<";
    if (lowered === "gt") return ">";
    if (lowered === "quot") return '"';
    if (lowered === "apos") return "'";

    if (lowered.startsWith("#x")) {
      const code = Number.parseInt(lowered.slice(2), 16);
      return Number.isFinite(code) ? String.fromCodePoint(code) : `&${entity};`;
    }
    if (lowered.startsWith("#")) {
      const code = Number.parseInt(lowered.slice(1), 10);
      return Number.isFinite(code) ? String.fromCodePoint(code) : `&${entity};`;
    }
    return `&${entity};`;
  });
}

function cleanTitle(raw: string): string {
  let value = decodeXmlEntities(raw).replace(/\s+/g, " ").trim();
  if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
    value = value.slice(1, -1).trim();
  }
  return value.length > 0 ? value : "(empty title)";
}

function getAttr(attrs: Record<string, string>, ...names: string[]): string | undefined {
  const lookup = new Set(names.map((name) => name.toLowerCase()));
  for (const [key, value] of Object.entries(attrs)) {
    if (lookup.has(key.toLowerCase())) {
      return value;
    }
  }
  return undefined;
}

function resolveElementPoint(node: ElementCtx): Point | undefined {
  const tag = node.name.toLowerCase();
  const x = parseNumeric(getAttr(node.attrs, "x", "cx", "x1"));
  const y = parseNumeric(getAttr(node.attrs, "y", "cy", "y1"));
  const width = parseNumeric(getAttr(node.attrs, "width"));
  const height = parseNumeric(getAttr(node.attrs, "height"));

  if ((tag === "image" || tag === "rect") && x !== undefined && y !== undefined) {
    if (width !== undefined && height !== undefined) {
      return { x: x + width, y };
    }
    return { x, y };
  }

  if ((tag === "text" || tag === "circle" || tag === "ellipse") && x !== undefined && y !== undefined) {
    return { x, y };
  }

  if (x !== undefined && y !== undefined) {
    return { x, y };
  }

  const points = parsePoints(getAttr(node.attrs, "points"));
  if (points) {
    return points;
  }

  return undefined;
}

function parseViewBox(value: string | undefined): { width?: number; height?: number } {
  if (!value) {
    return {};
  }
  const numbers = value
    .split(/[\s,]+/)
    .map((chunk) => Number.parseFloat(chunk))
    .filter((n) => Number.isFinite(n));
  if (numbers.length >= 4) {
    return { width: numbers[2], height: numbers[3] };
  }
  return {};
}

function parseMarkersWithSax(svgContent: string): ParsedSvg {
  const parser = sax.parser(false, {
    lowercase: true,
    trim: false,
    normalize: false,
    xmlns: false,
  });

  const stack: ElementCtx[] = [];
  const markers: RawMarker[] = [];
  let viewWidth: number | undefined;
  let viewHeight: number | undefined;

  parser.onopentag = (tag) => {
    const attrs: Record<string, string> = {};
    for (const [key, rawValue] of Object.entries(tag.attributes ?? {})) {
      if (typeof rawValue === "string") {
        attrs[key] = rawValue;
      } else {
        attrs[key] = String((rawValue as { value?: unknown }).value ?? "");
      }
    }

    const node: ElementCtx = {
      name: tag.name,
      attrs,
    };
    stack.push(node);

    if (node.name === "svg" && (viewWidth === undefined || viewHeight === undefined)) {
      const parsedViewBox = parseViewBox(getAttr(attrs, "viewbox"));
      viewWidth = parsedViewBox.width ?? parseNumeric(getAttr(attrs, "width"));
      viewHeight = parsedViewBox.height ?? parseNumeric(getAttr(attrs, "height"));
    }
  };

  parser.ontext = (text) => {
    const current = stack[stack.length - 1];
    if (current?.name === "title") {
      current.textContent = (current.textContent ?? "") + text;
    }
  };

  parser.oncdata = (text) => {
    const current = stack[stack.length - 1];
    if (current?.name === "title") {
      current.textContent = (current.textContent ?? "") + text;
    }
  };

  parser.onclosetag = () => {
    const node = stack.pop();
    if (!node) {
      return;
    }

    if (node.name === "title") {
      const parent = stack[stack.length - 1];
      if (parent && parent.titleText === undefined) {
        parent.titleText = node.textContent ?? "";
      }
      return;
    }

    const nodePoint = resolveElementPoint(node);
    const parent = stack[stack.length - 1];
    if (parent && nodePoint && parent.firstChildPoint === undefined) {
      parent.firstChildPoint = nodePoint;
    }

    if (node.titleText !== undefined && node.name !== "svg") {
      markers.push({
        index: markers.length + 1,
        title: cleanTitle(node.titleText),
        tag: node.name,
        point: nodePoint ?? node.firstChildPoint,
      });
    }
  };

  parser.write(svgContent).close();

  return {
    markers,
    viewWidth,
    viewHeight,
  };
}

function parseMarkers(svgContent: string): ParsedSvg {
  try {
    return parseMarkersWithSax(svgContent);
  } catch {
    const regex = /<([a-zA-Z_][a-zA-Z0-9:_-]*)([^>]*)>\s*<title>([\s\S]*?)<\/title>/g;
    const markers: RawMarker[] = [];
    let match: RegExpExecArray | null;
    while ((match = regex.exec(svgContent)) !== null) {
      const tag = match[1].toLowerCase();
      if (tag === "svg") {
        continue;
      }
      markers.push({
        index: markers.length + 1,
        tag,
        title: cleanTitle(match[3]),
      });
    }
    const viewBoxMatch = svgContent.match(/\bviewBox=["']([^"']+)["']/i);
    const parsedViewBox = parseViewBox(viewBoxMatch?.[1]);
    return {
      markers,
      viewWidth: parsedViewBox.width,
      viewHeight: parsedViewBox.height,
    };
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function projectMarkers(
  rawMarkers: RawMarker[],
  renderedWidth: number,
  renderedHeight: number,
  viewWidth?: number,
  viewHeight?: number,
): RenderedMarker[] {
  const scaleX = viewWidth && viewWidth > 0 ? renderedWidth / viewWidth : 1;
  const scaleY = viewHeight && viewHeight > 0 ? renderedHeight / viewHeight : 1;

  const radius = clamp(Math.round(renderedWidth / 160), 10, 18);
  const step = radius * 2 + 6;
  const cols = Math.max(1, Math.floor((renderedWidth - 20) / step));
  let fallbackIndex = 0;

  return rawMarkers.map((marker) => {
    let x: number;
    let y: number;

    if (marker.point) {
      x = marker.point.x * scaleX;
      y = marker.point.y * scaleY;
    } else {
      const col = fallbackIndex % cols;
      const row = Math.floor(fallbackIndex / cols);
      x = 10 + col * step;
      y = 10 + row * step;
      fallbackIndex += 1;
    }

    x = clamp(x, radius + 2, renderedWidth - radius - 2);
    y = clamp(y, radius + 2, renderedHeight - radius - 2);

    return {
      index: marker.index,
      title: marker.title,
      x,
      y,
    };
  });
}

function buildOverlaySvg(markers: RenderedMarker[], width: number, height: number): string {
  const radius = clamp(Math.round(width / 160), 10, 18);
  const textSizeBase = clamp(Math.round(radius * 0.95), 10, 15);

  const nodes = markers
    .map((marker) => {
      const text = String(marker.index);
      const textSize = text.length >= 3 ? Math.max(9, textSizeBase - 3) : textSizeBase;
      return `<g>
  <circle cx="${marker.x.toFixed(2)}" cy="${marker.y.toFixed(2)}" r="${radius}" fill="#d62828" stroke="white" stroke-width="1.5" />
  <text x="${marker.x.toFixed(2)}" y="${(marker.y + textSize * 0.35).toFixed(2)}" font-family="Arial, sans-serif" font-size="${textSize}" font-weight="700" fill="#ffffff" text-anchor="middle">${text}</text>
</g>`;
    })
    .join("\n");

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
${nodes}
</svg>`;
}

async function buildDocx(imagePng: Buffer, width: number, height: number, markers: RenderedMarker[]): Promise<Buffer> {
  const maxWidth = 900;
  const imageWidth = width > maxWidth ? maxWidth : width;
  const imageHeight = Math.max(1, Math.round((height / width) * imageWidth));

  const paragraphs: Paragraph[] = [
    new Paragraph({
      alignment: AlignmentType.CENTER,
      children: [
        new ImageRun({
          data: imagePng,
          type: "png",
          transformation: {
            width: imageWidth,
            height: imageHeight,
          },
        }),
      ],
    }),
    new Paragraph({ text: "" }),
  ];

  if (markers.length === 0) {
    paragraphs.push(new Paragraph({ children: [new TextRun("Элементов с title не найдено")] }));
  } else {
    for (const marker of markers) {
      paragraphs.push(new Paragraph({ children: [new TextRun(`${marker.index}. ${marker.title}`)] }));
    }
  }

  const doc = new Document({
    sections: [
      {
        children: paragraphs,
      },
    ],
  });

  return Packer.toBuffer(doc);
}

async function convertSvgToDocx(svgPath: string, outputPath: string): Promise<{ markers: number; encoding: string }> {
  const rawBuffer = await readFile(svgPath);
  const { content: svgContent, encoding } = decodeSvgBuffer(rawBuffer);
  const svgForRender = toUtf8Xml(svgContent);

  const parsed = parseMarkers(svgContent);

  const sourcePng = await sharp(Buffer.from(svgForRender, "utf8")).png().toBuffer();
  const metadata = await sharp(sourcePng).metadata();
  const width = metadata.width;
  const height = metadata.height;
  if (!width || !height) {
    throw new Error("Не удалось вычислить размер выходной картинки");
  }

  const renderedMarkers = projectMarkers(parsed.markers, width, height, parsed.viewWidth, parsed.viewHeight);
  const overlaySvg = buildOverlaySvg(renderedMarkers, width, height);
  const compositedPng = await sharp(sourcePng).composite([{ input: Buffer.from(overlaySvg) }]).png().toBuffer();

  const docxBuffer = await buildDocx(compositedPng, width, height, renderedMarkers);
  await writeFile(outputPath, docxBuffer);

  return {
    markers: renderedMarkers.length,
    encoding,
  };
}

async function runWithConcurrency<T>(
  items: string[],
  concurrency: number,
  worker: (item: string, index: number, total: number) => Promise<T>,
): Promise<T[]> {
  const results: T[] = new Array(items.length);
  let nextIndex = 0;

  const workers = Array.from({ length: Math.min(concurrency, items.length) }, async () => {
    while (true) {
      const current = nextIndex;
      nextIndex += 1;
      if (current >= items.length) {
        return;
      }
      results[current] = await worker(items[current], current, items.length);
    }
  });

  await Promise.all(workers);
  return results;
}

async function main(): Promise<void> {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const projectRoot = path.resolve(scriptDir, "..");
  const logFilePath = path.join(projectRoot, `${formatDateTimeForFileName(new Date())}.log`);
  const logger = new FileLogger(logFilePath);

  const options = parseArgs(process.argv.slice(2));
  const inputDir = path.resolve(options.inputDir);
  const outputDir = path.resolve(options.outputDir);

  logger.log("Process started");
  logger.log(`Input directory: ${inputDir}`);
  logger.log(`Output directory: ${outputDir}`);
  logger.log(`Concurrency: ${options.concurrency}`);

  await mkdir(outputDir, { recursive: true });
  const entries = await readdir(inputDir, { withFileTypes: true });
  let files = entries
    .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".svg"))
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b));

  if (options.match) {
    const needle = options.match.toLowerCase();
    files = files.filter((file) => file.toLowerCase().includes(needle));
  }
  if (options.limit) {
    files = files.slice(0, options.limit);
  }

  if (files.length === 0) {
    logger.log("No SVG files found for processing");
    await logger.close();
    console.log(`SVG-файлы не найдены. Лог: ${path.basename(logFilePath)}`);
    return;
  }

  logger.log(`Starting conversion. Files: ${files.length}`);

  sharp.concurrency(Math.max(1, Math.min(options.concurrency, 4)));

  const startedAt = Date.now();
  let completed = 0;
  let success = 0;
  let failed = 0;
  let totalMarkers = 0;
  let lastProgressLength = 0;

  const updateProgressLine = (): void => {
    const line = renderProgressBar(completed, files.length, success, failed, startedAt);
    const paddedLine = line.padEnd(lastProgressLength, " ");
    process.stdout.write(`\r${paddedLine}`);
    lastProgressLength = paddedLine.length;
  };

  updateProgressLine();

  await runWithConcurrency(files, options.concurrency, async (fileName, index, total) => {
    const svgPath = path.join(inputDir, fileName);
    const outName = `${path.parse(fileName).name}.docx`;
    const outputPath = path.join(outputDir, outName);
    const start = Date.now();

    try {
      const result = await convertSvgToDocx(svgPath, outputPath);
      success += 1;
      totalMarkers += result.markers;
      const elapsedMs = Date.now() - start;
      logger.log(
        `[${index + 1}/${total}] OK ${fileName} -> ${outName} | markers=${result.markers} | encoding=${result.encoding} | ${elapsedMs}ms`,
      );
    } catch (error) {
      failed += 1;
      const elapsedMs = Date.now() - start;
      const message = error instanceof Error ? error.message : String(error);
      logger.log(`[${index + 1}/${total}] FAIL ${fileName} | ${elapsedMs}ms | ${message}`);
    } finally {
      completed += 1;
      updateProgressLine();
    }
  });

  process.stdout.write("\n");
  const summary = `Готово. успешно=${success}, провально=${failed}, всего_маркеров=${totalMarkers}`;
  logger.log(summary);
  logger.log("Process finished");
  await logger.close();
  console.log(`${summary}. Лог: ${path.basename(logFilePath)}`);
}

await main();
