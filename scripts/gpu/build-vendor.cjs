"use strict";
const path = require("node:path"),
  fs = require("node:fs"),
  crypto = require("node:crypto");
const root = path.resolve(__dirname, "../.."),
  deps = path.join(root, "target/gpu-deps/node_modules"),
  esbuild = require(path.join(deps, "esbuild"));
(async () => {
  await esbuild.build({
    entryPoints: [path.join(__dirname, "vendor-entry.mjs")],
    outfile: path.join(root, "web/vendor/scene-three.mjs"),
    bundle: true,
    minify: true,
    format: "esm",
    platform: "browser",
    target: "es2022",
    alias: {
      three: path.join(deps, "three/build/three.webgpu.js"),
      "three/webgpu": path.join(deps, "three/build/three.webgpu.js"),
      "three/tsl": path.join(deps, "three/build/three.tsl.js"),
      "three/addons": path.join(deps, "three/examples/jsm"),
    },
    legalComments: "linked",
  });
  fs.copyFileSync(
    path.join(deps, "three/LICENSE"),
    path.join(root, "web/vendor/THREE-LICENSE.txt"),
  );
  fs.writeFileSync(
    path.join(root, "web/vendor/VERSIONS.json"),
    JSON.stringify(
      {
        three: "0.186.1",
        esbuild: "0.25.12",
        source: "https://github.com/mrdoob/three.js",
        sha256: crypto
          .createHash("sha256")
          .update(
            fs.readFileSync(path.join(root, "web/vendor/scene-three.mjs")),
          )
          .digest("hex"),
        command: "node scripts/gpu/build-vendor.cjs",
      },
      null,
      2,
    ) + "\n",
  );
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
