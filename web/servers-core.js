(function (root, factory) {
  const value = factory();
  if (typeof module === "object" && module.exports) module.exports = value;
  else root.SchemaServersCore = value;
})(typeof globalThis === "object" ? globalThis : this, () => {
  "use strict";
  const KEY = "schema-saved-servers-v1",
    LIMIT = 100;
  function parse(input) {
    if (typeof input !== "string" || input.length > 2048)
      throw Error("Enter a server address.");
    const text = input.trim();
    if (!text || /[\s\\\x00-\x1f\x7f]/.test(text) || text.startsWith("//"))
      throw Error(
        "Enter an HTTP(S) server address, without spaces or credentials.",
      );
    let url;
    try {
      url = new URL(
        /^[a-z][a-z0-9+.-]*:\/\//i.test(text) ? text : "http://" + text,
      );
    } catch {
      throw Error("Enter a valid address, such as schema.local:8787.");
    }
    const keys = [...url.searchParams.keys()];
    if (
      !["http:", "https:"].includes(url.protocol) ||
      url.username ||
      url.password ||
      !url.hostname ||
      url.hash ||
      ![
        "/",
        "/mobile",
        "/mobile.html",
        "/app",
        "/studio",
        "/index.html",
      ].includes(url.pathname) ||
      (keys.length &&
        (keys.length !== 1 ||
          keys[0] !== "view" ||
          url.searchParams.get("view") !== "studio" ||
          url.pathname.startsWith("/mobile")))
    )
      throw Error(
        "Use a server origin or Studio/schémaboy link, without passwords, tokens or extra parameters.",
      );
    // A bare unsupported scheme must never turn into a misleading HTTP hostname.
    if (
      /^[a-z][a-z0-9+.-]*:/i.test(text) &&
      !/^https?:\/\//i.test(text) &&
      !/^[-\w.]+:\d+(?:\/|$)/.test(text)
    )
      throw Error("Only HTTP and HTTPS addresses are supported.");
    return {
      origin: url.origin,
      view: url.pathname.startsWith("/mobile") ? "mobile" : "studio",
    };
  }
  function entry(value) {
    if (!value || typeof value !== "object") throw Error("Invalid server.");
    const parsed = parse(value.origin);
    const name =
      (typeof value.name === "string" ? value.name.trim() : "").slice(0, 80) ||
      new URL(parsed.origin).host;
    return {
      origin: parsed.origin,
      name,
      view: ["mobile", "studio"].includes(value.view)
        ? value.view
        : parsed.view,
    };
  }
  function link(value) {
    const server = entry(value);
    return (
      server.origin + (server.view === "mobile" ? "/mobile" : "/?view=studio")
    );
  }
  function repository(storage) {
    function list() {
      try {
        const value = JSON.parse(storage.getItem(KEY) || "null");
        if (value?.version !== 1 || !Array.isArray(value.servers)) return [];
        const seen = new Set(),
          servers = [];
        for (const item of value.servers.slice(0, LIMIT))
          try {
            const server = entry(item);
            if (!seen.has(server.origin)) {
              seen.add(server.origin);
              servers.push(server);
            }
          } catch {}
        return servers;
      } catch {
        return [];
      }
    }
    function write(servers) {
      try {
        storage.setItem(KEY, JSON.stringify({ version: 1, servers }));
      } catch {
        throw Error(
          "Could not save servers on this device. Check browser storage permissions or space.",
        );
      }
      return servers;
    }
    return {
      list,
      save(value) {
        const server = entry(value),
          old = list();
        if (old.length >= LIMIT && !old.some((v) => v.origin === server.origin))
          throw Error("Remove a saved server before adding another.");
        write([server, ...old.filter((v) => v.origin !== server.origin)]);
        return server;
      },
      remove(origin) {
        write(list().filter((v) => v.origin !== parse(origin).origin));
      },
    };
  }
  return { KEY, LIMIT, parse, entry, link, repository };
});
