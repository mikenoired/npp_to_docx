import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { decodeSvgBuffer, toUtf8Xml } from "./svg-parser.js";

/** Local resources only. Bound recursion and expansion for cyclic or excessive libraries. */
export async function prepareSvgResources(
  content: string,
  svgPath: string,
  ancestors: string[] = [],
  budget = { remaining: 2000 },
) {
  const missing = new Set<string>();
  const reconstructed = new Set<string>();
  let svg = content;
  const images = [...content.matchAll(/<image\b[^>]*>/gi)];
  for (const match of images) {
    const tag = match[0];
    const href = tag.match(/\b(?:xlink:href|href)\s*=\s*(["'])(.*?)\1/i);
    if (!href || href[2].startsWith("data:")) continue;
    const reference = href[2];
    let data: Buffer | undefined;
    let resourcePath = "";
    // Legacy drive paths are resolved by basename inside resource libraries only.
    const legacyDrivePath = /^[a-z]:[\\/]/i.test(reference);
    const localName = legacyDrivePath ? path.win32.basename(reference) : reference.replace(/\\/g, "/");
    const libraryRoots = [
      path.resolve(path.dirname(svgPath), "../submodels"),
      fileURLToPath(new URL("../../../assets/submodels/", import.meta.url)),
    ];
    if (!path.isAbsolute(localName) && !localName.split("/").includes("..") && !/^[a-z]+:/i.test(localName)) {
      const roots = legacyDrivePath ? libraryRoots : [path.dirname(svgPath), ...libraryRoots];
      for (const root of roots) {
        for (const candidate of [localName, `${localName}.svg`]) {
          try {
            resourcePath = path.join(root, /\.bmp$/i.test(candidate) ? `${candidate}.png` : candidate);
            data = await readFile(resourcePath);
            break;
          } catch {
            // Try the next local library before displaying a placeholder.
          }
        }
        if (data) break;
      }
    }
    let uri = "";
    if (data && /\.svg$/i.test(resourcePath)) {
      if (ancestors.includes(resourcePath) || ancestors.length >= 12 || --budget.remaining < 0) data = undefined;
      else {
        const decoded = toUtf8Xml(decodeSvgBuffer(data).content);
        if (/data-reconstructed="true"/.test(decoded)) reconstructed.add(reference);
        const nested = await prepareSvgResources(decoded, resourcePath, [...ancestors, resourcePath], budget);
        for (const name of nested.missingResources) missing.add(name);
        for (const name of nested.reconstructedResources) reconstructed.add(name);
        uri = `data:image/svg+xml;base64,${Buffer.from(nested.svg).toString("base64")}`;
      }
    } else if (data && /\.(png|jpe?g|bmp)$/i.test(reference)) {
      uri = `data:image/${/\.jpe?g$/i.test(reference) ? "jpeg" : "png"};base64,${data.toString("base64")}`;
    } else data = undefined;
    if (!data) {
      missing.add(reference);
      const placeholder =
        '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><rect x="1" y="1" width="98" height="98" fill="#fff4e5" stroke="#b66a00" stroke-width="2"/><path d="M5 5L95 95M95 5L5 95" stroke="#b66a00" stroke-width="2"/></svg>';
      uri = `data:image/svg+xml;base64,${Buffer.from(placeholder).toString("base64")}`;
    }
    svg = svg.replace(tag, tag.replace(href[0], `${href[0].slice(0, href[0].indexOf("="))}="${uri}"`));
  }
  return { svg, missingResources: [...missing].sort(), reconstructedResources: [...reconstructed].sort() };
}
