"use strict";
window.SchemaScenePreview = (() => {
  let adapter = {},
    pending = null;
  return {
    configure(value) {
      adapter = value;
    },
    async open() {
      try {
        if (!pending) pending = import("/scene-preview-ui.mjs");
        const module = await pending;
        await module.open(adapter);
      } catch (error) {
        pending = null;
        (adapter.status || alert)("Scene preview: " + error.message);
      }
    },
  };
})();
