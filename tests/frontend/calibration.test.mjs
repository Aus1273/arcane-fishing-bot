import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import ts from 'typescript';
// Exercise the same coordinate functions used by the pointer editor, without a browser.
const source = await readFile(
  new URL('../../src/lib/calibrationGeometry.ts', import.meta.url),
  'utf8',
);
const js = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
}).outputText;
const { boundRegion, drawnRegion, editRegion } = await import(
  `data:text/javascript;base64,${Buffer.from(js).toString('base64')}`
);
test('dragging clamps complete region to the screenshot without changing its size', () => {
  const region = { x: 150, y: 100, width: 300, height: 100 };
  assert.deepEqual(editRegion(region, { x: 150, y: 100 }, { x: -200, y: 1000 }, 'move', 800, 600), {
    x: 0,
    y: 500,
    width: 300,
    height: 100,
  });
});
test('drawing in reverse produces the same bounded pixel coordinates', () => {
  assert.deepEqual(drawnRegion({ x: 710, y: 500 }, { x: 310, y: 200 }, 800, 600), {
    x: 310,
    y: 200,
    width: 400,
    height: 300,
  });
});
test('resizing near the edge preserves anchor and clamps size', () => {
  const region = { x: 710, y: 500, width: 50, height: 50 };
  assert.deepEqual(
    editRegion(region, { x: 760, y: 550 }, { x: 1200, y: 1200 }, 'resize', 800, 600),
    { x: 710, y: 500, width: 90, height: 100 },
  );
  assert.deepEqual(editRegion(region, { x: 760, y: 550 }, { x: 0, y: 0 }, 'resize', 800, 600), {
    x: 710,
    y: 500,
    width: 1,
    height: 1,
  });
});
test('integer pixel regions never exceed a smaller available hotbar area', () => {
  assert.deepEqual(boundRegion({ x: 780, y: 25.8, width: 120.6, height: 10.2 }, 780, 600), {
    x: 659,
    y: 26,
    width: 121,
    height: 10,
  });
});
