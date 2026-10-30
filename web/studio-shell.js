"use strict";
(() => {
  if (!document.body.classList.contains("studio-app")) return;
  const media = matchMedia(
      "(max-width:900px), (max-width:1100px) and (max-height:500px)",
    ),
    header = document.querySelector("body>header"),
    actions = header.querySelector(".header-actions"),
    menu = document.createElement("details"),
    summary = document.createElement("summary"),
    items = document.createElement("div");
  menu.className = "studio-more-menu";
  summary.textContent = "⋯";
  summary.setAttribute("aria-label", "More Studio actions");
  summary.setAttribute("role", "button");
  items.className = "studio-more-items app-menu-panel";
  menu.append(summary, items);
  actions.append(menu);
  const primary = new Set(["undo", "redo", "save"]);
  for (const id of primary) {
    const button = document.getElementById(id),
      text = button.textContent;
    button.textContent = "";
    button.setAttribute(
      "aria-label",
      id === "save" ? "Save project" : id === "undo" ? "Undo" : "Redo",
    );
    const full = document.createElement("span");
    full.className = "studio-action-text";
    full.textContent = text;
    const compact = document.createElement("span");
    compact.className = "studio-action-compact";
    compact.textContent = id === "save" ? "Save" : id === "undo" ? "↶" : "↷";
    compact.setAttribute("aria-hidden", "true");
    button.append(full, compact);
  }
  const original = [];
  for (const id of ["scene-preview", "import", "export", "apply"]) {
    const node = document.getElementById(id),
      mark = document.createComment("Studio action");
    node.before(mark);
    original.push({ node, mark });
  }
  function collect() {
    for (const child of [...actions.children])
      if (child !== menu && !primary.has(child.id)) items.append(child);
  }
  function sync() {
    document.body.classList.toggle("studio-compact", media.matches);
    menu.open = !media.matches;
    for (const { node, mark } of original) {
      if (media.matches) items.append(node);
      else mark.after(node);
    }
    collect();
  }
  new MutationObserver(collect).observe(actions, { childList: true });
  media.addEventListener("change", sync);
  sync();
  function closeMenus(except) {
    for (const details of document.querySelectorAll(
      ".panel-menu[open],.workspace-menu[open],.studio-compact .studio-more-menu[open]",
    ))
      if (details !== except) details.open = false;
  }
  document.addEventListener("pointerdown", (event) =>
    closeMenus(
      event.target.closest(".panel-menu,.workspace-menu,.studio-more-menu"),
    ),
  );
  items.addEventListener("click", (event) => {
    if (media.matches && event.target.closest("button,a")) menu.open = false;
  });
  window.addEventListener(
    "keydown",
    (event) => {
      if (event.key !== "Escape" || document.querySelector("dialog[open]"))
        return;
      const details = document.querySelector(
        ".panel-menu[open],.workspace-menu[open],.studio-compact .studio-more-menu[open]",
      );
      if (details) {
        event.preventDefault();
        event.stopImmediatePropagation();
        closeMenus();
        details.querySelector("summary")?.focus();
      }
    },
    true,
  );
  addEventListener("resize", () => {
    if (media.matches) menu.open = false;
  });
})();
