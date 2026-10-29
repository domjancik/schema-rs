const assert = require('node:assert/strict'),
  F = require('../web/fold-motion.js');
const before = [
    { id: '1', x: 0, y: 112, width: 368, height: 112 },
    { id: '2', x: 0, y: 0, width: 178, height: 112 },
    { id: '3', x: 190, y: 0, width: 178, height: 112 }
  ],
  after = [
    { id: '1', x: 0, y: 112, width: 178, height: 112 },
    { id: '3', x: 0, y: 0, width: 178, height: 112 }
  ];
const effects = F.plan(before, after, { owner: '2', closing: true, deleting: true });
assert.equal(effects.filter((e) => e.kind === 'exit').length, 1);
const exit = effects.find((e) => e.kind === 'exit'),
  moves = effects.filter((e) => e.kind === 'move');
assert(exit.duration <= 200);
assert(moves.every((e) => e.delay >= exit.duration));
assert(effects.every((e) => e.duration + e.delay <= 550));
assert.equal(exit.frames.at(-1).opacity, 0);
console.log('Deletion exit precedes bounded restack motion');
