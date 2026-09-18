import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { deflateSync } from "node:zlib";

import {
  Canvas,
  brailleChar,
  brailleMask,
  renderBytes,
  renderFile,
  renderLuma,
  renderRgb,
  renderRgba,
  version,
} from "../index.js";

const ESC = String.fromCharCode(27);
const BLANK = "⠀";
const FULL = "⣿";

/** Encodes an RGBA PNG without pulling in an image library. */
function makePng(width, height, pixel) {
  const raw = [];
  for (let y = 0; y < height; y++) {
    raw.push(0); // filter: none
    for (let x = 0; x < width; x++) {
      raw.push(...pixel(x, y));
    }
  }

  const crcTable = [];
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    crcTable[n] = c >>> 0;
  }
  const crc32 = (buffer) => {
    let c = 0xffffffff;
    for (const byte of buffer) {
      c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
    }
    return (c ^ 0xffffffff) >>> 0;
  };

  const chunk = (kind, payload) => {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(payload.length);
    const body = Buffer.concat([Buffer.from(kind, "ascii"), payload]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body));
    return Buffer.concat([length, body, crc]);
  };

  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.writeUInt8(8, 8); // bit depth
  header.writeUInt8(6, 9); // colour type: RGBA

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(Buffer.from(raw))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const checkerboard = makePng(32, 32, (x, y) => {
  const value = (Math.floor(x / 4) + Math.floor(y / 4)) % 2 === 0 ? 255 : 0;
  return [value, value, value, 255];
});

const isBraille = (text) =>
  text.length > 0 &&
  [...text].every(
    (c) => c === "\n" || (c.codePointAt(0) >= 0x2800 && c.codePointAt(0) <= 0x28ff),
  );

test("version is exposed", () => {
  assert.match(version(), /^\d+\.\d+\.\d+/);
});

test("renderBytes produces Braille", () => {
  const art = renderBytes(checkerboard, { width: 16 });
  assert.ok(isBraille(art));
  for (const line of art.split("\n")) {
    assert.equal([...line].length, 16);
  }
});

test("renderFile matches renderBytes", async () => {
  const dir = await mkdtemp(join(tmpdir(), "rustille-"));
  const path = join(dir, "checkerboard.png");
  await writeFile(path, checkerboard);
  assert.equal(renderFile(path, { width: 16 }), renderBytes(checkerboard, { width: 16 }));
});

test("renderRgba renders raw pixels", () => {
  const width = 16;
  const height = 16;
  const rgba = new Uint8Array(width * height * 4).fill(255);
  const art = renderRgba(rgba, width, height, { width: 8 });
  assert.equal(art.split("\n")[0], FULL.repeat(8));
});

test("renderRgb and renderLuma agree with renderRgba", () => {
  const width = 8;
  const height = 8;
  const luma = new Uint8Array(width * height);
  for (let i = 0; i < luma.length; i++) {
    luma[i] = (i * 4) % 256;
  }
  const rgb = new Uint8Array(width * height * 3);
  const rgba = new Uint8Array(width * height * 4);
  for (let i = 0; i < luma.length; i++) {
    rgb.set([luma[i], luma[i], luma[i]], i * 3);
    rgba.set([luma[i], luma[i], luma[i], 255], i * 4);
  }
  const options = { width: 4, fit: "stretch" };
  const fromLuma = renderLuma(luma, width, height, options);
  assert.equal(fromLuma, renderRgb(rgb, width, height, options));
  assert.equal(fromLuma, renderRgba(rgba, width, height, options));
});

test("options change the output", () => {
  const plain = renderBytes(checkerboard, { width: 8 });
  assert.notEqual(renderBytes(checkerboard, { width: 8, invert: true }), plain);

  const allLit = renderBytes(checkerboard, { width: 8, threshold: 0 });
  assert.ok([...allLit].every((c) => c === FULL || c === "\n"));

  assert.ok(isBraille(renderBytes(checkerboard, { width: 8, dither: "floyd-steinberg" })));
  assert.ok(renderBytes(checkerboard, { width: 8, color: "truecolor" }).includes(`${ESC}[38;2;`));
  assert.ok(!plain.includes(ESC));
  assert.equal(renderBytes(checkerboard, { height: 4 }).split("\n").length, 4);
  assert.equal(
    renderBytes(checkerboard, { width: 8, height: 8, fit: "stretch" }).split("\n").length,
    8,
  );
});

test("transparent pixels take the background colour", () => {
  const transparent = makePng(8, 8, () => [255, 255, 255, 0]);
  const options = { width: 2, height: 1, fit: "stretch" };
  assert.equal(renderBytes(transparent, options), BLANK.repeat(2));
  assert.equal(
    renderBytes(transparent, { ...options, background: "white" }),
    FULL.repeat(2),
  );
});

test("errors are thrown, not returned", () => {
  assert.throws(() => renderFile("definitely-not-here.png"));
  assert.throws(() => renderBytes(Buffer.from("not an image")));
  assert.throws(() => renderRgba(new Uint8Array(3), 2, 2));
  assert.throws(() => renderBytes(checkerboard, { dither: "atkinson" }));
  assert.throws(() => renderBytes(checkerboard, { width: 0 }));
});

test("canvas draws and renders", () => {
  const canvas = new Canvas(100, 50);
  assert.equal(canvas.width, 100);
  assert.equal(canvas.height, 50);
  assert.equal(canvas.cellsWidth, 50);
  assert.equal(canvas.cellsHeight, 13);

  assert.equal(canvas.set(1, 1), true);
  assert.equal(canvas.get(1, 1), true);
  assert.equal(canvas.set(1000, 1000), false);
  assert.equal(canvas.count(), 1);

  canvas.unset(1, 1);
  assert.equal(canvas.get(1, 1), false);

  canvas.line(0, 0, 99, 49);
  canvas.rectangle(0, 0, 99, 49);
  canvas.filledRectangle(10, 10, 20, 20);
  canvas.circle(50, 25, 10);
  canvas.filledCircle(50, 25, 3);
  assert.ok(canvas.count() > 0);

  const rendered = canvas.render();
  assert.equal(rendered.split("\n").length, 13);
  assert.ok(isBraille(rendered));

  canvas.fill();
  assert.equal(canvas.count(), 100 * 50);
  canvas.clear();
  assert.equal(canvas.count(), 0);

  assert.throws(() => new Canvas(0xffffffff, 0xffffffff));
});

test("braille helpers round-trip", () => {
  assert.equal(brailleChar(0), BLANK);
  assert.equal(brailleChar(255), FULL);
  assert.equal(brailleMask(FULL), 255);
  assert.equal(brailleMask("a"), null);
  for (let mask = 0; mask < 256; mask++) {
    assert.equal(brailleMask(brailleChar(mask)), mask);
  }
});
