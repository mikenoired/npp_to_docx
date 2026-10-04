import { readFile } from "node:fs/promises";
import path from "node:path";

import { decodeSvgBuffer, toUtf8Xml } from "./svg-parser.js";

/** Resolve local images only; never fetch network resources from an uploaded frame. */
export async function prepareSvgResources(content: string, svgPath: string) {
  const missing = new Set<string>();
  let svg = content;
  const images = [...content.matchAll(/<image\b[^>]*>/gi)];
  for (const match of images) {
    const tag = match[0];
    const href = tag.match(/\b(?:xlink:href|href)\s*=\s*(["'])(.*?)\1/i);
    if (!href || href[2].startsWith("data:")) continue;
    const reference = href[2];
    let data: Buffer | undefined;
    const localName = reference.replace(/\\/g, "/");
    if (!path.isAbsolute(localName) && !localName.split("/").includes("..") && !/^[a-z]+:/i.test(localName)) {
      try {
        data = await readFile(path.join(path.dirname(svgPath), localName));
      } catch {
        // An absent submodel must remain visible in the passport.
      }
    }
    let uri = "";
    if (data && /\.svg$/i.test(reference)) {
      const decoded = toUtf8Xml(decodeSvgBuffer(data).content);
      // Nested external links are unsupported: expose the missing resource instead of a partial symbol.
      if (/<image\b[^>]*\b(?:xlink:href|href)\s*=\s*["'](?!data:)/i.test(decoded)) data = undefined;
      else uri = `data:image/svg+xml;base64,${Buffer.from(decoded).toString("base64")}`;
    } else if (data && /\.(png|jpe?g)$/i.test(reference)) {
      uri = `data:image/${/\.png$/i.test(reference) ? "png" : "jpeg"};base64,${data.toString("base64")}`;
    } else data = undefined;
    if (!data) {
      missing.add(reference);
      const placeholder =
        '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><rect x="1" y="1" width="98" height="98" fill="#fff4e5" stroke="#b66a00" stroke-width="2"/><path d="M5 5L95 95M95 5L5 95" stroke="#b66a00" stroke-width="2"/></svg>';
      uri = `data:image/svg+xml;base64,${Buffer.from(placeholder).toString("base64")}`;
    }
    svg = svg.replace(tag, tag.replace(href[0], `${href[0].slice(0, href[0].indexOf("="))}="${uri}"`));
  }
  return { svg, missingResources: [...missing].sort() };
}
