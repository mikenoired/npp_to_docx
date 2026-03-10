import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

type EmbeddedAsset = {
  contentType: string;
  encoding: "utf8" | "base64";
  body: string;
};

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const buildDir = path.join(projectRoot, "build/web-client");
const outputFile = path.join(projectRoot, "src/web/server/embedded-assets.ts");

function getContentType(filePath: string): string {
  const extension = path.extname(filePath);
  switch (extension) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
      return "text/javascript; charset=utf-8";
    case ".css":
      return "text/css; charset=utf-8";
    case ".json":
      return "application/json; charset=utf-8";
    case ".svg":
      return "image/svg+xml";
    case ".png":
      return "image/png";
    case ".jpg":
    case ".jpeg":
      return "image/jpeg";
    case ".woff2":
      return "font/woff2";
    default:
      return "application/octet-stream";
  }
}

async function collectFiles(directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(
    entries.map(async (entry) => {
      const resolved = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        return collectFiles(resolved);
      }
      return [resolved];
    }),
  );
  return nested.flat();
}

function toAssetKey(filePath: string): string {
  const relative = path.relative(buildDir, filePath).split(path.sep).join("/");
  return relative === "index.html" ? "/index.html" : `/${relative}`;
}

async function main(): Promise<void> {
  const files = await collectFiles(buildDir);
  const assets: Record<string, EmbeddedAsset> = {};

  for (const filePath of files) {
    const buffer = await readFile(filePath);
    const key = toAssetKey(filePath);
    const contentType = getContentType(filePath);
    const isText =
      contentType.startsWith("text/") || contentType.includes("javascript") || contentType.includes("json");

    assets[key] = isText
      ? {
          contentType,
          encoding: "utf8",
          body: buffer.toString("utf8"),
        }
      : {
          contentType,
          encoding: "base64",
          body: buffer.toString("base64"),
        };
  }

  const source = `export type EmbeddedAsset = {
  contentType: string;
  encoding: "utf8" | "base64";
  body: string;
};

export const embeddedAssets: Record<string, EmbeddedAsset> = ${JSON.stringify(assets, null, 2)};\n`;

  await writeFile(outputFile, source, "utf8");
  console.log(`Embedded ${files.length} web assets into ${path.relative(projectRoot, outputFile)}`);
}

await main();
