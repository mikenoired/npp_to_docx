import { strict as assert } from "node:assert";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import iconv from "iconv-lite";
import { decodeSvgBuffer, parseMarkers, toUtf8Xml } from "./svg-parser.js";
import { prepareSvgResources } from "./svg-resources.js";

const frame =
  '<?xml version="1.0" encoding="UTF-8"?><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:rt="urn:rt" width="100" height="100"><title>Насос</title><image x="10" y="20" width="30" height="40" xlink:href="DS_motor.svg"><title>41ABC</title><rt:dyn type="KKS" value="41ABC"/></image></svg>';
const decoded = decodeSvgBuffer(iconv.encode(frame, "windows-1251"));
assert.equal(decoded.encoding, "windows-1251");
assert.ok(decoded.content.includes("Насос"));
assert.equal(decodeSvgBuffer(Buffer.from(frame)).encoding, "utf-8");
assert.equal(parseMarkers(decoded.content).markers[0].kks, "41ABC");
assert.equal(parseMarkers(frame.replace("<title>41ABC</title>", "")).markers[0].kks, "41ABC");
assert.ok(toUtf8Xml("<svg><rt:model/>&#4;</svg>").includes("xmlns:rt="));
assert.ok(!toUtf8Xml("<svg>&#4;&#x4;</svg>").includes("&#"));
const dir = await mkdtemp(path.join(tmpdir(), "npp-passport-"));
try {
  const missing = await prepareSvgResources(frame, path.join(dir, "frame.svg"));
  assert.deepEqual(missing.missingResources, ["DS_motor.svg"]);
  assert.ok(missing.svg.includes("data:image/svg+xml;base64,"));
  await writeFile(
    path.join(dir, "DS_motor.svg"),
    '<svg xmlns="http://www.w3.org/2000/svg"><rect width="30" height="40"/></svg>',
  );
  assert.deepEqual((await prepareSvgResources(frame, path.join(dir, "frame.svg"))).missingResources, []);
  assert.deepEqual(
    (await prepareSvgResources(frame.replace("DS_motor.svg", "https://example.com/a.svg"), path.join(dir, "frame.svg")))
      .missingResources,
    ["https://example.com/a.svg"],
  );
} finally {
  await rm(dir, { recursive: true, force: true });
}
console.log("Passport regression checks passed");
