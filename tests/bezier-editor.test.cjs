"use strict";
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs');
test('scalar Bezier uses value controls at fixed thirds, supports overshoot, and preserves endpoints',()=>{
 assert(fs.existsSync('web/bezier-editor.js'),'Shared visual scalar Bezier editor is available');
 const B=require('../web/bezier-editor.js');
 assert.equal(B.value([0,0,0,1],.5),.125);
 assert.equal(B.value([-2,3,4,8],0),-2);assert.equal(B.value([-2,3,4,8],1),8);
 assert(B.value([0,3,3,1],.5)>1);
 const range=B.range([0,0,1,1]);assert(range[0]<0&&range[1]>1);
 assert.deepEqual(B.range([5,5,5,5]),[4,6]);
});
