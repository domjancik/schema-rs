"use strict";
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path"),
  http = require("node:http"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const root = path.resolve("target/browser-site"),
  types = {
    ".html": "text/html",
    ".js": "text/javascript",
    ".mjs": "text/javascript",
    ".wasm": "application/wasm",
    ".json": "application/json",
    ".css": "text/css",
  };
const server = http.createServer((req, res) => {
  const url = new URL(req.url, "http://localhost");
  if (url.pathname === "/test") {
    res.setHeader("Content-Type", "text/html");
    res.end("<!doctype html><title>Migration test</title>");
    return;
  }
  const file = path.resolve(root, "." + url.pathname);
  if (!file.startsWith(root + path.sep) || !fs.existsSync(file)) {
    res.writeHead(404);
    res.end();
    return;
  }
  const config = JSON.parse(
    fs.readFileSync("scripts/publication/vercel.json", "utf8"),
  );
  for (const header of (
    config.headers.find((h) => h.source === url.pathname) || config.headers[0]
  ).headers)
    res.setHeader(header.key, header.value);
  res.setHeader(
    "Content-Type",
    types[path.extname(file)] || "application/octet-stream",
  );
  fs.createReadStream(file).pipe(res);
});
const sessionText = fs.readFileSync(
    "tests/fixtures/storage-v1/session-late.json",
    "utf8",
  ),
  project = JSON.parse(
    fs.readFileSync("tests/fixtures/storage-v1/project-early.json", "utf8"),
  );
(async () => {
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const base = "http://127.0.0.1:" + server.address().port,
    b = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const page = await b.newPage();
    await page.goto(base + "/test");
    await page.evaluate(
      async ({ sessionText, project }) => {
        await new Promise((resolve, reject) => {
          const request = indexedDB.open("schema-browser", 1);
          request.onupgradeneeded = () => {
            for (const name of ["sessions", "projects", "resources"])
              request.result.createObjectStore(name, { keyPath: "id" });
          };
          request.onerror = () => reject(request.error);
          request.onsuccess = () => {
            const db = request.result,
              tx = db.transaction(["sessions", "projects"], "readwrite");
            tx.objectStore("sessions").put({ id: "legacy", text: sessionText });
            tx.objectStore("projects").put({
              id: "old-project",
              title: "Original saved project",
              project,
              fixtures: 1,
              saved_revision: 7,
              modified_unix_ms: 123,
              legacy: false,
            });
            tx.oncomplete = () => {
              db.close();
              resolve();
            };
          };
        });
      },
      { sessionText, project },
    );
    await page.evaluate(async () => {
      const { SchemaPlayer } = await import("/schema-player.mjs");
      window.legacyPlayer = await SchemaPlayer.create({
        persist: true,
        autoTick: false,
        sessionId: "legacy",
      });
    });
    const migrated = await page.evaluate(() =>
      legacyPlayer.request("/api/session"),
    );
    assert.equal(migrated.version, 2);
    const original = JSON.parse(sessionText);
    for (const key of ["undo", "redo"]) {
      assert.equal(migrated[key].length, original[key].length);
      for (let i = 0; i < migrated[key].length; i++)
        assert.equal(migrated[key][i].version, 2);
    }
    for (const key of ["states", "inputs", "plugins", "resources", "snapshot"])
      assert.deepEqual(migrated[key], original[key], key + " retained");
    assert(migrated.checkpoints["Before layout edit"]);
    async function records() {
      return page.evaluate(
        () =>
          new Promise((resolve, reject) => {
            const r = indexedDB.open("schema-browser");
            r.onerror = () => reject(r.error);
            r.onsuccess = () => {
              const db = r.result,
                tx = db.transaction(
                  ["sessions", "projects", "resources", "migration-backups"],
                  "readonly",
                ),
                data = { version: db.version };
              for (const name of [
                "sessions",
                "projects",
                "resources",
                "migration-backups",
              ]) {
                const q = tx.objectStore(name).getAll();
                q.onsuccess = () => (data[name] = q.result);
              }
              tx.oncomplete = () => {
                db.close();
                resolve(data);
              };
            };
          }),
      );
    }
    let stored = await records();
    assert.equal(stored.version, 2);
    assert.equal(
      stored.sessions[0].text,
      sessionText,
      "Loading leaves the original record unchanged",
    );
    assert.equal(stored["migration-backups"].length, 0);
    await page.evaluate(() => legacyPlayer.request("/api/session/save", {}));
    stored = await records();
    assert.equal(JSON.parse(stored.sessions[0].text).version, 2);
    const exact=text=>JSON.parse(text,(_key,value,context)=>typeof value==='number'&&!Number.isSafeInteger(value)&&/^\d+$/.test(context?.source||'')?BigInt(context.source):value);
    assert.deepEqual(exact(stored.sessions[0].text).states,exact(sessionText).states,'64-bit random state is preserved exactly without a JavaScript object roundtrip');

    assert.equal(
      stored["migration-backups"].find((b) => b.store === "sessions").original
        .text,
      sessionText,
      "Exact raw source record retained",
    );
    await page.evaluate(async () => {
      const state = await legacyPlayer.request("/api/state");
      await legacyPlayer.request("/api/projects/save", {
        id: "old-project",
        title: "Migrated saved project",
        overwrite: true,
        expected_revision: state.snapshot.revision,
      });
    });
    stored = await records();
    assert.equal(JSON.parse(stored.projects[0].project_text).version, 2);
    assert.deepEqual(
      stored["migration-backups"].find((b) => b.store === "projects").original
        .project,
      project,
    );

    await page.evaluate(
      async ({ project, sessionText, migrated }) => {
        const rows = [
          { id: "old-layout", value: { fixtures: project.fixtures } },
          {
            id: "old-content",
            value: [
              {
                fixture: "fixture-1",
                layer: 0,
                stack: project.content["fixture-1"][0],
              },
            ],
          },
        ];
        await new Promise((resolve) => {
          const r = indexedDB.open("schema-browser");
          r.onsuccess = () => {
            const db = r.result,
              tx = db.transaction(["resources", "sessions"], "readwrite");
            for (const row of rows)
              tx.objectStore("resources").put({
                ...row,
                id:
                  row.id === "old-layout"
                    ? "layout:old-layout"
                    : "content:old-content",
              });
            tx.objectStore("sessions").put({
              id: "rollback",
              text: sessionText,
            });
            tx.oncomplete = () => {
              db.close();
              resolve();
            };
          };
        });
        await legacyPlayer.request("/api/project/save", {
          name: "old-layout",
          scope: "layout",
        });
        await legacyPlayer.request("/api/project/save", {
          name: "old-content",
          scope: "content",
        });
        const { openStorage } = await import("/browser-storage.mjs"),
          storage = await openStorage(
            await legacyPlayer.request("/api/storage/versions"),
          );
        let failed = false;
        try {
          await storage.saveSession(
            {
              id: "rollback",
              text: JSON.stringify(migrated),
              unclonable: () => {},
            },
            sessionText,
          );
        } catch {
          failed = true;
        }
        if (!failed) throw Error("Injected IndexedDB failure did not abort");
        const record={id:'race',project_text:JSON.stringify(migrated.project)};
        await storage.saveProject(record,true,null);
        let raced=false;
        try{await storage.saveProject(record,true,null);}catch{raced=true;}
        if(!raced)throw Error('A concurrent project creation was overwritten');
        storage.db.close();
      },
      { project, sessionText, migrated },
    );
    stored = await records();
    assert(stored.resources.every((r) => JSON.parse(r.value_text).version === 2));
    assert.equal(
      stored["migration-backups"].filter((b) => b.store === "resources").length,
      2,
    );
    assert.equal(
      stored.sessions.find((r) => r.id === "rollback").text,
      sessionText,
    );
    assert(
      !stored["migration-backups"].some((b) => b.key === "rollback"),
      "Failed replacement rolls back the backup transaction too",
    );
    const future = structuredClone(migrated);
    future.version = 99;
    const futureText = JSON.stringify(future);
    await page.evaluate(async (futureText) => {
      await new Promise((resolve) => {
        const r = indexedDB.open("schema-browser");
        r.onsuccess = () => {
          const db = r.result,
            tx = db.transaction("sessions", "readwrite");
          tx.objectStore("sessions").put({ id: "legacy", text: futureText });
          tx.oncomplete = () => {
            db.close();
            resolve();
          };
        };
      });
    }, futureText);
    await assert.rejects(
      page.evaluate(() => legacyPlayer.request("/api/session/save", {})),
      /Unsupported|another editor/,
    );
    assert.equal((await records()).sessions[0].text, futureText);
    await page.evaluate(() => legacyPlayer.dispose());
    await assert.rejects(
      page.evaluate(async () => {
        const { SchemaPlayer } = await import("/schema-player.mjs");
        await SchemaPlayer.create({
          persist: true,
          autoTick: false,
          sessionId: "legacy",
        });
      }),
      /future|unsupported/i,
    );
    assert.equal((await records()).sessions[0].text, futureText);

    await page.evaluate(
      () =>
        new Promise((resolve) => {
          const r = indexedDB.open("schema-browser", 3);
          r.onupgradeneeded = () => r.result.createObjectStore("future-data");
          r.onsuccess = () => {
            const db = r.result,
              tx = db.transaction("future-data", "readwrite");
            tx.objectStore("future-data").put("preserve", "sentinel");
            tx.oncomplete = () => {
              db.close();
              resolve();
            };
          };
        }),
    );
    await assert.rejects(
      page.evaluate(async () => {
        const { SchemaPlayer } = await import("/schema-player.mjs");
        await SchemaPlayer.create({
          persist: true,
          autoTick: false,
          sessionId: "future-db",
        });
      }),
      /newer than this app/i,
    );
    assert.equal(
      await page.evaluate(
        () =>
          new Promise((resolve) => {
            const r = indexedDB.open("schema-browser");
            r.onsuccess = () => {
              const db = r.result,
                q = db
                  .transaction("future-data")
                  .objectStore("future-data")
                  .get("sentinel");
              q.onsuccess = () => {
                const result = q.result;
                db.close();
                resolve(result);
              };
            };
          }),
      ),
      "preserve",
    );
    console.log(
      "Actual WASM: v1 session/project migration, complete history/runtime retention, atomic IndexedDB backups, stale writes and future-version refusal passed.",
    );
  } finally {
    await b.close();
    await new Promise((r) => server.close(r));
  }
})().catch((e) => {
  console.error(e);
  server.close();
  process.exitCode = 1;
});
