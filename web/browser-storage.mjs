// IndexedDB schema upgrades add stores only. Rust owns document migrations and validation.
export const DATABASE_VERSION = 3;
export function openStorage(versions) {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("schema-browser", DATABASE_VERSION);
    let blocked = false;
    request.onupgradeneeded = () => {
      for (const name of [
        "sessions",
        "projects",
        "resources",
        "library",
        "migration-backups",
      ])
        if (!request.result.objectStoreNames.contains(name))
          request.result.createObjectStore(name, { keyPath: "id" });
    };
    request.onsuccess = () => {
      if (blocked) {
        request.result.close();
        return;
      }
      const storage = new BrowserStorage(request.result, versions);
      request.result.onversionchange = () => request.result.close();
      resolve(storage);
    };
    request.onerror = () =>
      reject(
        request.error?.name === "VersionError"
          ? Error(
              "Browser storage is newer than this app. Open a newer app version; existing data was retained.",
            )
          : request.error,
      );
    request.onblocked = () => {
      blocked = true;
      reject(
        Error(
          "Close older browser editors to upgrade project storage. Existing data was retained.",
        ),
      );
    };
  });
}
class BrowserStorage {
  constructor(db, versions) {
    this.db = db;
    this.versions = {...versions, library:1};
  }
  access(store, mode, action) {
    return new Promise((resolve, reject) => {
      let tx;
      try {
        tx = this.db.transaction(store, mode);
      } catch (error) {
        reject(error);
        return;
      }
      let result;
      const request = action(tx.objectStore(store));
      request.onsuccess = () => (result = request.result);
      request.onerror = () => reject(request.error);
      tx.oncomplete = () => resolve(result);
      tx.onabort = () =>
        reject(tx.error || Error("Storage transaction aborted"));
      tx.onerror = () => reject(tx.error);
    });
  }
  get(store, id) {
    return this.access(store, "readonly", (s) => s.get(id));
  }
  all(store) {
    return this.access(store, "readonly", (s) => s.getAll());
  }
  put(store, value) {
    if (store === "sessions") return this.saveSession(value);
    if (store === "resources") return this.saveResource(value);
    return this.access(store, "readwrite", (s) => s.put(value));
  }
  saveSession(record, expectedText, forceBackup = false) {
    return this.replace("sessions", record, { expectedText, forceBackup });
  }
  saveLibrary(record, expected) { return this.replace("library",record,{overwrite:!!expected,expected,forceBackup:true}); }
  saveProject(record, overwrite, expected) {
    return this.replace("projects", record, { overwrite, expected });
  }
  saveResource(record, expected) {
    return this.replace("resources", record, { expected });
  }
  format(store, record) {
    if(store === "library") return {kind:"library",version:record.item.version};
    if (store === "sessions") {
      const document = JSON.parse(record.text);
      return { kind: "session", version: document.version };
    }
    if (store === "projects")
      return { kind: "project", version: (record.project_text ? JSON.parse(record.project_text) : record.project)?.version };
    const kind = record.id.split(":")[0];
    if (!["layout", "content"].includes(kind))
      throw Error("Unknown versioned browser resource");
    const value = record.value_text ? JSON.parse(record.value_text) : record.value;
    return { kind, version: Array.isArray(value) ? 1 : (value?.version ?? 1) };
  }
  replace(store, record, options = {}) {
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction([store, "migration-backups"], "readwrite"),
        target = tx.objectStore(store),
        backups = tx.objectStore("migration-backups"),
        request = target.get(record.id);
      let failure;
      const check = (record) => {
        const result = this.format(store, record),
          current = this.versions?.[result.kind];
        if (!Number.isInteger(current) || current < 1)
          throw Error("Storage versions are unavailable");
        if (
          !Number.isInteger(result.version) ||
          result.version < 1 ||
          result.version > current
        )
          throw Error(
            "Unsupported " +
              result.kind +
              " storage version " +
              result.version +
              "; original data was retained",
          );
        return { ...result, current };
      };
      request.onsuccess = () => {
        try {
          const old = request.result,
            latest = check(record);
          if (latest.version !== latest.current)
            throw Error("Migrate and validate the document before saving");
          if (old) {
            if (options.overwrite === false)
              throw Error("Project exists; select it to overwrite.");
            const source = check(old);
            if (
              options.expectedText !== undefined &&
              old.text !== options.expectedText
            )
              throw Error(
                "Session changed in another editor; reload before saving",
              );
            if (
              options.expected !== undefined &&
              JSON.stringify(old) !== JSON.stringify(options.expected)
            )
              throw Error(
                "Saved data changed in another editor; reload before saving",
              );
            if (source.version < source.current || options.forceBackup)
              backups.add({
                id: store + ":" + record.id + ":" + crypto.randomUUID(),
                store,
                key: record.id,
                from_version: source.version,
                to_version: source.current,
                created_ms: Date.now(),
                original: old,
              });
          } else if (
            (options.expectedText !== undefined &&
              options.expectedText !== null) ||
            options.expected
          )
            throw Error(
              "Saved data was removed in another editor; reload before saving",
            );
          target.put(record);
        } catch (error) {
          failure = error;
          tx.abort();
        }
      };
      tx.oncomplete = () => resolve(record);
      tx.onabort = () =>
        reject(failure || tx.error || Error("Storage transaction aborted"));
      tx.onerror = () => reject(tx.error);
    });
  }
}
