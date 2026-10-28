const assert = require('node:assert/strict'),
  F = require('../web/fold-motion.js');
const old = [
    { id: '1', x: 0, y: 224, width: 558, height: 112, opacity: 1 },
    { id: '2', x: 0, y: 112, width: 178, height: 112, opacity: 1 },
    { id: '3', x: 190, y: 112, width: 368, height: 112, opacity: 1 }
  ],
  next = [{ id: '1', x: 380, y: 224, width: 178, height: 112, opacity: 1 }];
const close = F.plan(old, next, { owner: '1', closing: true });
assert.equal(close.filter((p) => p.kind === 'exit').length, 2);
assert(close.some((p) => p.id === '1' && p.kind === 'move'));
assert(close.every((p) => p.duration + p.delay <= 450));
const open = F.plan(next, old, { owner: '1', closing: false });
assert.equal(open.filter((p) => p.kind === 'enter').length, 2);
assert(open.find((p) => p.id === '2').frames[0].opacity < 1);
assert(open.find((p) => p.id === '2').frames.at(-1).opacity === 1);
assert.equal(F.plan(old, old, { owner: '1', closing: false }).length, 0);
const many = Array.from({ length: 500 }, (_, i) => ({
  id: String(i),
  x: i * 190,
  y: 112,
  width: 178,
  height: 112,
  opacity: 1
}));
assert(F.plan(many, [], { owner: '0', closing: true }).length <= 96);
assert.deepEqual(
  F.anchor({ x: 100, y: 200, scale: 2 }, { right: 600, top: 300 }, { x: 0, y: 0, width: 178 }),
  { x: 144, y: 100 }
);


assert.equal(
  F.plan(many.slice(0, 1), many, {
    owner: '0',
    closing: false,
    knownBeforeIds: many.map((n) => n.id)
  }).length,
  0,
  'Cards outside the capture budget must not animate as new entries'
);

console.log('Fold motion entering/exiting/sibling plans, pinned anchor and bounded timing passed');
