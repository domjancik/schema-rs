"use strict";
SchemaScenePreview.configure({ page: true, embedded: parent !== window });
SchemaScenePreview.open();

if (parent !== window) {
  let maximized = false;
  const panel = new URLSearchParams(location.search).get("panel");
  addEventListener(
    "keydown",
    (event) => {
      if (
        (event.ctrlKey && event.code === "Space") ||
        (event.key === "Escape" && maximized)
      ) {
        event.preventDefault();
        event.stopImmediatePropagation();
        parent.postMessage(
          {
            type:
              event.key === "Escape"
                ? "schema-panel-restore"
                : "schema-panel-maximize",
            panel,
          },
          location.origin,
        );
      }
    },
    true,
  );
  addEventListener("message", (event) => {
    if (
      event.origin !== location.origin ||
      event.source !== parent ||
      event.data?.type !== "schema-panel-visibility"
    )
      return;
    maximized = !!event.data.maximized;
    import("./scene-preview-ui.mjs").then((module) =>
      module.setActive(!!event.data.active),
    );
  });
}
