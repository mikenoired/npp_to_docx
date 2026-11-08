import { strict as assert } from "node:assert";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import iconv from "iconv-lite";
import { decodeSvgBuffer, parseMarkers, toUtf8Xml } from "./svg-parser.js";
import { prepareSvgResources } from "./svg-resources.js";

const frame =
  '<?xml version="1.0" encoding="UTF-8"?><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:rt="urn:rt" width="100" height="100"><title>Насос</title><image x="10" y="20" width="30" height="40" xlink:href="fixture_motor.svg"><title>41ABC</title><rt:dyn type="KKS" value="41ABC"/></image></svg>';
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
  assert.deepEqual(missing.missingResources, ["fixture_motor.svg"]);
  assert.ok(missing.svg.includes("data:image/svg+xml;base64,"));
  await mkdir(path.join(dir, "frames"));
  await mkdir(path.join(dir, "submodels"));
  await writeFile(
    path.join(dir, "submodels", "fixture_motor.svg"),
    '<svg xmlns="http://www.w3.org/2000/svg"><rect width="30" height="40"/></svg>',
  );
  assert.deepEqual((await prepareSvgResources(frame, path.join(dir, "frames", "frame.svg"))).missingResources, []);
  await writeFile(
    path.join(dir, "fixture_motor.svg"),
    '<svg xmlns="http://www.w3.org/2000/svg"><rect width="30" height="40"/></svg>',
  );
  assert.deepEqual((await prepareSvgResources(frame, path.join(dir, "frame.svg"))).missingResources, []);
  await writeFile(
    path.join(dir, "submodels", "outer.svg"),
    '<svg xmlns="http://www.w3.org/2000/svg"><image href="fixture_motor.svg"/></svg>',
  );
  const outerFrame = frame.replace("fixture_motor.svg", "outer.svg");
  assert.deepEqual((await prepareSvgResources(outerFrame, path.join(dir, "frames", "frame.svg"))).missingResources, []);
  await writeFile(
    path.join(dir, "submodels", "outer.svg"),
    '<svg xmlns="http://www.w3.org/2000/svg"><image href="outer.svg"/></svg>',
  );
  assert.deepEqual((await prepareSvgResources(outerFrame, path.join(dir, "frames", "frame.svg"))).missingResources, [
    "outer.svg",
  ]);
  assert.deepEqual(
    (
      await prepareSvgResources(
        frame.replace("fixture_motor.svg", "../fixture_motor.svg"),
        path.join(dir, "frames", "frame.svg"),
      )
    ).missingResources,
    ["../fixture_motor.svg"],
  );
  assert.deepEqual(
    (
      await prepareSvgResources(
        frame.replace("fixture_motor.svg", "https://example.com/a.svg"),
        path.join(dir, "frame.svg"),
      )
    ).missingResources,
    ["https://example.com/a.svg"],
  );
  const bundled = await prepareSvgResources(
    frame.replace("fixture_motor.svg", "DS_CO_valve.svg"),
    path.join(dir, "frames", "frame.svg"),
  );
  assert.deepEqual(bundled.missingResources, []);
  assert.deepEqual(bundled.reconstructedResources, []);
  const rebuilt = await prepareSvgResources(
    frame.replace("fixture_motor.svg", "DS_ana_900.svg"),
    path.join(dir, "frames", "frame.svg"),
  );
  assert.deepEqual(rebuilt.missingResources, []);
  assert.deepEqual(rebuilt.reconstructedResources, ["DS_ana_900.svg"]);
  const legacyReference = "F:\\old\\документ_ЦВД_низ.jpg";
  const legacy = await prepareSvgResources(
    frame.replace("fixture_motor.svg", legacyReference),
    path.join(dir, "frames", "frame.svg"),
  );
  assert.deepEqual(legacy.missingResources, []);
  assert.deepEqual(legacy.reconstructedResources, [legacyReference]);
} finally {
  await rm(dir, { recursive: true, force: true });
}
console.log("Passport regression checks passed");
