import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { resolve } from "node:path";

const pkgDir = resolve(process.argv[2] ?? "./pkg-node");
const require = createRequire(import.meta.url);
const rustille = require(pkgDir);

const FULL = "⣿";

console.log(`rustille-wasm ${rustille.version()}`);

const width = 16;
const height = 16;
const rgba = new Uint8Array(width * height * 4).fill(255);
const art = rustille.renderRgba(rgba, width, height, { width: 8 });
assert.equal(art.split("\n")[0], FULL.repeat(8));
console.log("renderRgba:");
console.log(art);

const dithered = rustille.renderRgba(
  new Uint8Array(width * height * 4).fill(110),
  width,
  height,
  { width: 8, dither: "floyd-steinberg" },
);
assert.ok(dithered.length > 0);
assert.throws(() => rustille.renderRgba(rgba, width, height, { dither: "atkinson" }));
assert.throws(() => rustille.renderRgba(new Uint8Array(3), 2, 2));

if (typeof rustille.renderBytes === "function") {
  const png = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR42mP4DwQACfsD/Wj6HMwAAAAASUVORK5CYII=",
    "base64",
  );
  const decoded = rustille.renderBytes(png, { width: 1, height: 1, fit: "stretch" });
  assert.equal(decoded, FULL);
  console.log("renderBytes: ok");
}

const canvas = new rustille.Canvas(60, 24);
canvas.rectangle(0, 0, 59, 23);
canvas.circle(30, 12, 9);
canvas.line(0, 0, 59, 23);
assert.equal(canvas.cellsWidth, 30);
assert.equal(canvas.cellsHeight, 6);
assert.ok(canvas.count() > 0);
console.log("canvas:");
console.log(canvas.render());
canvas.free();

for (let mask = 0; mask < 256; mask++) {
  assert.equal(rustille.brailleMask(rustille.brailleChar(mask)), mask);
}

console.log("wasm smoke test ok");
