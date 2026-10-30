"use strict";
const test = require("node:test"),
  assert = require("node:assert/strict");
const C = require("../web/servers-core.js");
const memory = () => {
  const values = new Map();
  return {
    getItem: (k) => values.get(k) || null,
    setItem: (k, v) => values.set(k, v),
  };
};
test("pairing accepts host origins and direct host UI QR links without credentials", () => {
  assert.deepEqual(C.parse("schema.local:8787"), {
    origin: "http://schema.local:8787",
    view: "studio",
  });
  assert.deepEqual(C.parse(" https://lights.example/mobile "), {
    origin: "https://lights.example",
    view: "mobile",
  });
  assert.equal(
    C.parse("http://[fe80::1]:8787/?view=studio").origin,
    "http://[fe80::1]:8787",
  );
  for (const bad of [
    "javascript:alert(1)",
    "file:///etc/passwd",
    "https://user:secret@schema.local",
    "http://schema.local/?token=1234",
    "http://schema.local/#1234",
    "//evil.example",
    "http://schema.local/path",
    "http://schema.local/\\evil",
    "http://schema.local/?view=studio&pin=1",
    "data:text/html,hello",
  ])
    assert.throws(() => C.parse(bad), bad);
});
test("saved servers deduplicate canonical origins and survive reload without secrets", () => {
  const storage = memory(),
    r = C.repository(storage);
  r.save({ origin: "HTTP://Schema.Local:80/", name: "Desk", view: "studio" });
  r.save({
    origin: "http://schema.local/mobile",
    name: "Phone",
    view: "mobile",
    token: "never",
  });
  assert.deepEqual(C.repository(storage).list(), [
    { origin: "http://schema.local", name: "Phone", view: "mobile" },
  ]);
  assert.equal(C.link(r.list()[0]), "http://schema.local/mobile");
  r.remove("http://schema.local");
  assert.deepEqual(r.list(), []);
});
test("malformed storage and quota errors are handled explicitly", () => {
  assert.deepEqual(C.repository({ getItem: () => "{broken" }).list(), []);
  const storage = memory();
  storage.setItem(
    C.KEY,
    JSON.stringify({
      version: 1,
      servers: [
        { origin: "javascript:alert(1)", name: "Bad", view: "studio" },
        {
          origin: "http://ok.local",
          name: "<img onerror=alert(1)>",
          view: "studio",
        },
      ],
    }),
  );
  assert.equal(C.repository(storage).list().length, 1);
  assert.throws(
    () =>
      C.repository({
        getItem: () => null,
        setItem: () => {
          throw Error("quota");
        },
      }).save({ origin: "http://ok.local" }),
    /save|storage/i,
  );
});

test("generated pairing QR decodes as a direct host UI URL", () => {
  const encode = require("../web/vendor/qrcode.js"),
    decode = require("../web/vendor/jsQR.js");
  const url = C.link({ origin: "http://schema.local:8787", view: "mobile" }),
    qr = encode(0, "M");
  qr.addData(url);
  qr.make();
  const cells = qr.getModuleCount(),
    scale = 8,
    size = (cells + 8) * scale,
    data = new Uint8ClampedArray(size * size * 4);
  data.fill(255);
  for (let r = 0; r < cells; r++)
    for (let c = 0; c < cells; c++)
      if (qr.isDark(r, c))
        for (let y = 0; y < scale; y++)
          for (let x = 0; x < scale; x++) {
            const at = (((r + 4) * scale + y) * size + (c + 4) * scale + x) * 4;
            data[at] = data[at + 1] = data[at + 2] = 0;
          }
  const found = decode(data, size, size);
  assert.equal(found.data, url);
  assert.deepEqual(C.parse(found.data), {
    origin: "http://schema.local:8787",
    view: "mobile",
  });
});
