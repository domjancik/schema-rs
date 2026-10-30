"use strict";
window.SchemaServers = (() => {
  const C = window.SchemaServersCore,
    loads = new Map();
  let storage;
  try {
    storage = localStorage;
  } catch {
    storage = {
      getItem: () => null,
      setItem: () => {
        throw Error("Storage unavailable");
      },
    };
  }
  const repo = C.repository(storage);
  let dialog,
    address,
    name,
    view,
    message,
    list,
    qrArea,
    scanArea,
    video,
    stream,
    timer,
    serial = 0,
    visible = false;
  const el = (tag, text, cls) => {
    const e = document.createElement(tag);
    if (text !== undefined) e.textContent = text;
    if (cls) e.className = cls;
    return e;
  };
  const button = (text, action) => {
    const b = el("button", text);
    b.type = "button";
    b.onclick = action;
    return b;
  };
  function status(text) {
    message.textContent = text;
  }
  function loadScript(path, global) {
    if (window[global]) return Promise.resolve(window[global]);
    if (!loads.has(path))
      loads.set(
        path,
        new Promise((resolve, reject) => {
          const script = el("script");
          script.src = path;
          script.onload = () => resolve(window[global]);
          script.onerror = () => {
            loads.delete(path);
            script.remove();
            reject(Error("QR tools could not load. Reconnect and try again."));
          };
          document.head.append(script);
        }),
      );
    return loads.get(path);
  }
  function field(text, input) {
    const label = el("label", text);
    input.setAttribute("aria-label", text);
    label.append(input);
    return label;
  }
  function stopCamera() {
    serial++;
    clearTimeout(timer);
    stream?.getTracks().forEach((track) => track.stop());
    stream = null;
    if (video) {
      video.pause();
      video.srcObject = null;
    }
    if (scanArea) scanArea.hidden = true;
  }
  function review(text) {
    const parsed = C.parse(text);
    address.value = parsed.origin;
    view.value = parsed.view;
    name.value = new URL(parsed.origin).host;
    status("QR read. Review the address, then save this server.");
    address.focus();
  }
  async function decode(source, width, height, max = 1600) {
    const decoder = await loadScript("/vendor/jsQR.js", "jsQR");
    const scale = Math.min(1, max / Math.max(width, height)),
      canvas = el("canvas");
    canvas.width = Math.max(1, Math.round(width * scale));
    canvas.height = Math.max(1, Math.round(height * scale));
    const context = canvas.getContext("2d", { willReadFrequently: true });
    context.drawImage(source, 0, 0, canvas.width, canvas.height);
    const image = context.getImageData(0, 0, canvas.width, canvas.height);
    return decoder(image.data, image.width, image.height, {
      inversionAttempts: "attemptBoth",
    })?.data;
  }
  async function photo(file) {
    stopCamera();
    if (!file) return;
    const request = serial;
    try {
      if (
        file.size > 8 * 1024 * 1024 ||
        !["image/png", "image/jpeg", "image/webp", "image/gif"].includes(
          file.type,
        )
      )
        throw Error("Choose a PNG, JPEG, WebP or GIF image up to 8 MB.");
      status("Reading QR image…");
      const bitmap = await createImageBitmap(file);
      let text;
      try {
        if (bitmap.width * bitmap.height > 32000000)
          throw Error("Choose a smaller QR image.");
        text = await decode(bitmap, bitmap.width, bitmap.height);
      } finally {
        bitmap.close();
      }
      if (request !== serial || !visible) return;
      if (!text)
        throw Error(
          "No QR code found. Try a clearer image or enter the address.",
        );
      review(text);
    } catch (error) {
      if (request === serial && visible) status(error.message);
    }
  }
  async function camera() {
    stopCamera();
    const request = serial;
    try {
      if (!isSecureContext || !navigator.mediaDevices?.getUserMedia)
        throw Error(
          "Camera scanning needs HTTPS or localhost. Choose a QR image or enter the address.",
        );
      await loadScript("/vendor/jsQR.js", "jsQR");
      if (request !== serial || !visible) return;
      const result = await navigator.mediaDevices.getUserMedia({
        video: {
          facingMode: { ideal: "environment" },
          width: { ideal: 640 },
          height: { ideal: 480 },
        },
        audio: false,
      });
      if (request !== serial || !visible || document.hidden) {
        result.getTracks().forEach((t) => t.stop());
        return;
      }
      stream = result;
      scanArea.hidden = false;
      video.srcObject = stream;
      await video.play();
      status("Point the camera at a server QR code.");
      async function tick() {
        if (request !== serial || !visible) return;
        try {
          if (video.videoWidth) {
            const text = await decode(
              video,
              video.videoWidth,
              video.videoHeight,
              640,
            );
            if (request !== serial) return;
            if (text) {
              stopCamera();
              review(text);
              return;
            }
          }
        } catch (error) {
          stopCamera();
          status(error.message);
          return;
        }
        timer = setTimeout(tick, 250);
      }
      tick();
    } catch (error) {
      if (request === serial) {
        stopCamera();
        status(
          error.name === "NotAllowedError"
            ? "Camera permission was not granted. Choose a QR image or enter the address."
            : error.message,
        );
      }
    }
  }
  async function showQR(server) {
    stopCamera();
    const request = serial;
    try {
      const encode = await loadScript("/vendor/qrcode.js", "qrcode");
      if (!visible || request !== serial) return;
      const url = C.link(server),
        code = encode(0, "M");
      code.addData(url);
      code.make();
      const cells = code.getModuleCount(),
        scale = Math.max(3, Math.floor(320 / (cells + 8))),
        canvas = el("canvas");
      canvas.width = canvas.height = (cells + 8) * scale;
      canvas.setAttribute("aria-label", "Pairing QR code");
      canvas.setAttribute("role", "img");
      const ctx = canvas.getContext("2d");
      ctx.fillStyle = "#fff";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.fillStyle = "#000";
      for (let row = 0; row < cells; row++)
        for (let col = 0; col < cells; col++)
          if (code.isDark(row, col))
            ctx.fillRect((col + 4) * scale, (row + 4) * scale, scale, scale);
      const link = el("input");
      link.value = url;
      link.readOnly = true;
      link.setAttribute("aria-label", "Pairing link");
      const download = el("a", "Download QR");
      download.href = canvas.toDataURL("image/png");
      download.download = "schema-server.png";
      download.className = "server-action";
      const copy = button("Copy link", async () => {
        try {
          await navigator.clipboard.writeText(url);
          status("Server link copied.");
        } catch {
          link.focus();
          link.select();
          status("Copy the selected server link.");
        }
      });
      qrArea.replaceChildren(
        el("h3", "Pair another device"),
        canvas,
        link,
        copy,
        download,
        el(
          "p",
          "Scan this to open the server’s own UI. Login stays on that server; the QR contains no PIN or token.",
        ),
      );
      if (
        ["localhost", "127.0.0.1", "[::1]"].includes(
          new URL(server.origin).hostname,
        )
      )
        qrArea.append(
          el(
            "p",
            "This address only works on this device. Use the server’s LAN address to pair a phone.",
            "server-warning",
          ),
        );
      qrArea.hidden = false;
      qrArea.tabIndex = -1;
      qrArea.focus({ preventScroll: true });
      qrArea.scrollIntoView({ block: "start", behavior: "instant" });
    } catch (error) {
      status(error.message);
    }
  }
  function render() {
    list.replaceChildren();
    const servers = repo.list();
    if (!servers.length)
      list.append(
        el(
          "p",
          "No saved servers yet. Add an address or scan a QR code.",
          "server-empty",
        ),
      );
    for (const server of servers) {
      const row = el("article", undefined, "saved-server"),
        head = el("div", undefined, "server-row-heading");
      head.append(el("strong", server.name), el("span", server.origin));
      const preferred = el("select");
      preferred.append(
        new Option("Studio", "studio"),
        new Option("schémaboy", "mobile"),
      );
      preferred.value = server.view;
      preferred.setAttribute("aria-label", "View for " + server.name);
      const open = el(
        "a",
        server.view === "mobile" ? "Open schémaboy" : "Open Studio",
      );
      open.href = C.link(server);
      open.target = "_blank";
      open.rel = "noopener noreferrer";
      open.className = "server-action primary";
      preferred.onchange = () => {
        try {
          repo.save({ ...server, view: preferred.value });
          render();
        } catch (error) {
          status(error.message);
        }
      };
      const actions = el("div", undefined, "server-row-actions");
      actions.append(
        preferred,
        open,
        button("QR", () => showQR(server)),
        button("Edit", () => {
          address.value = server.origin;
          name.value = server.name;
          view.value = server.view;
          address.focus();
        }),
        button("Remove", () => {
          try {
            repo.remove(server.origin);
            render();
            status("Server removed from this device.");
          } catch (error) {
            status(error.message);
          }
        }),
      );
      row.append(head, actions);
      list.append(row);
    }
  }
  function create() {
    const page = document.querySelector("[data-servers-page]");
    dialog = el(page ? "section" : "dialog");
    dialog.className = "servers-dialog";
    dialog.id = "servers-dialog";
    dialog.setAttribute("aria-labelledby", "servers-title");
    const heading = el("header"),
      title = el("h2", "Servers");
    title.id = "servers-title";
    heading.append(title);
    if (!page)
      heading.append(
        button("×", () => {
          visible = false;
          stopCamera();
          dialog.close();
        }),
      );
    heading.lastElementChild.setAttribute(
      "aria-label",
      page ? "Servers" : "Close servers",
    );
    const intro = el(
      "p",
      "Open a server’s own Studio or schémaboy. Saved addresses stay on this device; each server handles its own login.",
    );
    const form = el("form");
    form.className = "server-form";
    address = el("input");
    address.type = "text";
    address.inputMode = "url";
    address.autocomplete = "url";
    address.maxLength = 2048;
    address.required = true;
    address.addEventListener("change", () => {
      try {
        const parsed = C.parse(address.value);
        if (
          parsed.view === "mobile" ||
          /[?&]view=studio(?:&|$)/.test(address.value)
        )
          view.value = parsed.view;
      } catch {}
    });
    address.placeholder = "schema.local:8787";
    name = el("input");
    name.maxLength = 80;
    name.autocomplete = "off";
    view = el("select");
    view.append(
      new Option("Studio", "studio"),
      new Option("schémaboy", "mobile"),
    );
    view.value =
      new URLSearchParams(location.search).get("view") === "mobile" ||
      location.pathname.startsWith("/mobile")
        ? "mobile"
        : "studio";
    const submit = el("button", "Save server");
    submit.type = "submit";
    submit.className = "primary";
    form.append(
      field("Server address", address),
      field("Server name", name),
      field("Preferred view", view),
      submit,
    );
    form.onsubmit = (event) => {
      event.preventDefault();
      try {
        const parsed = C.parse(address.value);
        repo.save({ ...parsed, name: name.value, view: view.value });
        address.value = "";
        name.value = "";
        render();
        status("Server saved. Open it when you are ready.");
      } catch (error) {
        status(error.message);
      }
    };
    const scan = button("Scan QR", camera),
      image = el("input");
    image.type = "file";
    image.accept = "image/png,image/jpeg,image/webp,image/gif";
    image.setAttribute("aria-label", "Choose QR image");
    image.onchange = () => {
      photo(image.files[0]);
      image.value = "";
    };
    const pairing = el("div", undefined, "server-pairing");
    pairing.append(scan, field("Choose QR image", image));
    scanArea = el("section", undefined, "server-scanner");
    scanArea.hidden = true;
    video = el("video");
    video.muted = true;
    video.playsInline = true;
    video.setAttribute("aria-label", "QR camera");
    scanArea.append(
      video,
      button("Stop camera", () => {
        stopCamera();
        status("Camera stopped.");
      }),
    );
    message = el("p");
    message.className = "server-status";
    message.setAttribute("role", "status");
    message.setAttribute("aria-live", "polite");
    list = el("div", undefined, "saved-servers");
    qrArea = el("section", undefined, "server-qr");
    qrArea.hidden = true;
    const content = el("div", undefined, "server-content");
    dialog.append(heading, content);
    content.append(
      intro,
      form,
      pairing,
      scanArea,
      message,
      el("h3", "Saved servers"),
      list,
      qrArea,
      el(
        "p",
        "Opening a host uses its own interface version and may open a browser window.",
        "server-hint",
      ),
    );
    if (
      !page &&
      document.documentElement.dataset.engine !== "browser" &&
      window.SchemaEngine?.kind !== "browser" &&
      !document.body.hasAttribute("data-offline")
    ) {
      const current = {
        origin: location.origin,
        name: location.host,
        view: location.pathname.startsWith("/mobile") ? "mobile" : "studio",
      };
      const here = el("div", undefined, "server-current");
      here.append(
        el("h3", "This server"),
        el("p", current.origin),
        button("Save this server", () => {
          try {
            repo.save(current);
            render();
            status("Current server saved.");
          } catch (error) {
            status(error.message);
          }
        }),
        button("Share this server QR", () => showQR(current)),
      );
      content.append(here);
    }
    const updates = el("section", undefined, "server-updates"),
      updateStatus = el(
        "p",
        "Reloading the app keeps saved servers and projects. It does not update a remote engine.",
      ),
      check = button("Check for app updates", async () => {
        check.disabled = true;
        updateStatus.textContent = "Checking…";
        try {
          const result = await SchemaAppUpdates.check();
          updateStatus.textContent =
            result.message + (result.version ? " · " + result.version : "");
          reload.textContent = result.available
            ? "Update app now"
            : "Reload app";
        } catch (error) {
          updateStatus.textContent = error.message;
        } finally {
          check.disabled = false;
        }
      }),
      reload = button("Reload app", async () => {
        reload.disabled = true;
        try {
          await SchemaAppUpdates.apply();
        } catch (error) {
          updateStatus.textContent = error.message;
          reload.disabled = false;
        }
      });
    updateStatus.setAttribute("role", "status");
    updates.append(el("h3", "App updates"), updateStatus, check, reload);
    content.append(updates);
    if (page) page.append(dialog);
    else {
      document.body.append(dialog);
      dialog.addEventListener("close", () => {
        if (!dialog.open) {
          visible = false;
          stopCamera();
        }
      });
      dialog.addEventListener("cancel", () => {
        visible = false;
        stopCamera();
      });
    }
    addEventListener("pagehide", stopCamera);
    document.addEventListener("visibilitychange", () => {
      if (document.hidden && stream) {
        stopCamera();
        status("Camera stopped while the app is hidden.");
      }
    });
  }
  function open() {
    if (!dialog) create();
    visible = true;
    qrArea.hidden = true;
    stopCamera();
    render();
    status("");
    if (dialog.tagName === "DIALOG" && !dialog.open) dialog.showModal();
    try {
      if (!repo.list().length) {
        const old = storage.getItem("schema-public-engine");
        if (old) {
          address.value = C.parse(old).origin;
          status("Save your previously used server to keep it here.");
        }
      }
    } catch {}
  }
  const host =
    document.querySelector(".app-menu-panel") ||
    document.querySelector(".header-actions");
  if (host) host.append(button("Servers", open));
  document
    .querySelectorAll("[data-open-servers]")
    .forEach((b) => b.addEventListener("click", open));
  if (document.querySelector("[data-servers-page]")) open();
  addEventListener("storage", (e) => {
    if (e.key === C.KEY && visible) render();
  });
  return { open };
})();
