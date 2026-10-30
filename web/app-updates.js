"use strict";
window.SchemaAppUpdates = (() => {
  const loaded =
    document.querySelector('meta[name="schema-app-build"]')?.content || "";
  const browser = () =>
    document.documentElement.dataset.engine === "browser" ||
    document.documentElement.dataset.appHost === "browser" ||
    window.SchemaEngine?.kind === "browser";
  let registration = null;
  async function check() {
    if ("serviceWorker" in navigator) {
      registration = await navigator.serviceWorker.getRegistration();
      await registration?.update();
      const worker = registration?.installing;
      if (worker && !["installed", "activated"].includes(worker.state)) {
        await new Promise((resolve, reject) => {
          const timer = setTimeout(() => {
            worker.removeEventListener("statechange", change);
            reject(
              Error(
                "The update is still downloading. Check again in a moment.",
              ),
            );
          }, 8000);
          function change() {
            if (["installed", "activated"].includes(worker.state)) {
              clearTimeout(timer);
              worker.removeEventListener("statechange", change);
              resolve();
            } else if (worker.state === "redundant") {
              clearTimeout(timer);
              worker.removeEventListener("statechange", change);
              reject(
                Error(
                  "The update could not be downloaded. Reconnect and try again.",
                ),
              );
            }
          }
          worker.addEventListener("statechange", change);
          change();
        });
      }
    }
    if (!browser())
      return {
        available: !!registration?.waiting,
        message:
          "The app comes from this host. Reload to use the interface it currently serves. Engine updates are managed on the host.",
      };
    const response = await fetch("/browser-build.json", {
      cache: "no-store",
      signal: AbortSignal.timeout(8000),
    });
    if (!response.ok)
      throw Error("Could not check for updates. Reconnect and try again.");
    const value = await response.json();
    if (!/^[a-f0-9]{40}$/.test(value.source_commit || ""))
      throw Error("The update information is invalid.");
    const available =
      !!registration?.waiting || (!!loaded && loaded !== value.source_commit);
    return {
      available,
      message: available
        ? "An app update is ready. Reload when you are ready."
        : loaded
          ? "You are using the latest app."
          : "Reload to load the latest app.",
      version: value.source_commit.slice(0, 7),
    };
  }
  async function apply() {
    await window.SchemaAppNavigation?.prepare();
    if (!registration && "serviceWorker" in navigator)
      registration = await navigator.serviceWorker.getRegistration();
    const worker = registration?.waiting;
    if (worker) {
      await new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          worker.removeEventListener("statechange", change);
          reject(Error("The app update did not activate. Try again."));
        }, 8000);
        function change() {
          if (worker.state === "activated") {
            clearTimeout(timer);
            worker.removeEventListener("statechange", change);
            resolve();
          } else if (worker.state === "redundant") {
            clearTimeout(timer);
            worker.removeEventListener("statechange", change);
            reject(Error("The update was replaced. Check again."));
          }
        }
        worker.addEventListener("statechange", change);
        worker.postMessage({ type: "SCHEMA_ACTIVATE_UPDATE" });
        change();
      });
    }
    location.reload();
  }
  return { check, apply, loaded };
})();
