import {
  navigateCamera,
  cameraPreset,
  validateCamera,
} from "./scene-camera.mjs";
import {
  THREE as T,
  OrbitControls,
  GLTFLoader,
  RectAreaLightTexturesLib,
  pass,
  mrt,
  output,
  normalView,
  diffuseColor,
  velocity,
  traa,
  packNormalToRGB,
  unpackRGBToNormal,
  sample,
  ssgi,
  vec4,
  uniform,
  screenUV,
  screenCoordinate,
  bayer16,
  gaussianBlur,
} from "./vendor/scene-three.mjs";
const VOLUME = 10,
  QUALITY = {
    low: {
      lights: 12,
      ratio: 0.7,
      steps: 6,
      slices: 2,
      giSteps: 6,
      shadows: 0,
      fps: 20,
    },
    balanced: {
      lights: 40,
      ratio: 1,
      steps: 8,
      slices: 2,
      giSteps: 6,
      shadows: 2,
      fps: 30,
    },
    high: {
      lights: 64,
      ratio: 1.35,
      steps: 14,
      slices: 3,
      giSteps: 10,
      shadows: 6,
      fps: 60,
    },
  };
const disposeTree = (root) =>
  root.traverse((object) => {
    object.geometry?.dispose();
    for (const material of [object.material].flat().filter(Boolean)) {
      for (const value of Object.values(material))
        if (value?.isTexture) value.dispose();
      material.dispose();
    }
  });
export class SceneView {
  static async create(canvas, options = {}) {
    const view = new SceneView(canvas, options);
    try {
      await view.init();
      return view;
    } catch (error) {
      view.dispose();
      throw error;
    }
  }
  constructor(
    canvas,
    {
      backend = "auto",
      quality = "balanced",
      onSelect = () => {},
      onError = () => {},
      onCameraChange = () => {},
    } = {},
  ) {
    this.canvas = canvas;
    this.options = { backend, quality };
    this.onSelect = onSelect;
    this.onError = onError;
    this.onCameraChange = onCameraChange;
    this.navigationMode = "orbit";
    this.settings = {
      gi: true,
      volumes: true,
      haze: 0.18,
      gain: 12,
      exposure: 1.4,
      ambient: 0.13,
      layout: true,
      emitters: true,
    };
    this.stats = {
      frames: 0,
      lights: 0,
      sourceCount: 0,
      fixtures: 0,
      pixels: 0,
      backend: "initializing",
      fps: 0,
    };
    this.data = null;
    this.lightSignature = "";
    this.active = false;
    this.geometry = new T.Group();
    this.sourceLights = new T.Group();
    this.raycaster = new T.Raycaster();
    this.pointer = new T.Vector2();
    this.picked = [];
    this.layouts = [];
    this.matrix = new T.Object3D();
    this.color = new T.Color();
    this.listeners = [];
    this.geometryGeneration = 0;
  }
  async init() {
    const renderer = (this.renderer = new T.WebGPURenderer({
      canvas: this.canvas,
      antialias: false,
      alpha: false,
      forceWebGL: this.options.backend === "webgl",
    }));
    renderer.setPixelRatio(1);
    renderer.toneMapping = T.ACESFilmicToneMapping;
    renderer.toneMappingExposure = this.settings.exposure;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = T.PCFShadowMap;
    await renderer.init();
    if (!this.canvas.isConnected)
      throw Error("Preview initialization superseded");
    this.stats.backend = renderer.backend.isWebGPUBackend ? "WebGPU" : "WebGL2";
    this.advanced =
      renderer.backend.isWebGPUBackend ||
      !!renderer.backend.gl?.getExtension("EXT_color_buffer_float");
    this.stats.effects = this.advanced
      ? "advanced"
      : "basic (no float targets)";
    if (!this.advanced) renderer.shadowMap.enabled = false;
    this.scene = new T.Scene();
    this.scene.background = new T.Color("#10171b");
    this.camera = new T.PerspectiveCamera(48, 1, 0.03, 300);
    this.camera.position.set(7, 5, 8);
    this.controls = new OrbitControls(this.camera, this.canvas);
    this.controls.enableDamping = true;
    this.controls.target.set(0, 1, 0);
    this.controls.maxDistance = 150;
    this.controls.minDistance = 0.2;
    this.canvas.tabIndex = 0;
    this.canvas.setAttribute(
      "aria-keyshortcuts",
      "ArrowLeft ArrowRight ArrowUp ArrowDown W A S D + - F Home",
    );
    this.cameraChanged = () =>
      this.onCameraChange({ ...this.cameraState(), mode: this.navigationMode });
    this.controls.addEventListener("change", this.cameraChanged);
    this.listeners.push(() =>
      this.controls.removeEventListener("change", this.cameraChanged),
    );
    this.listen(this.canvas, "keydown", (event) => {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      const key = event.key.toLowerCase(),
        pan = event.shiftKey || this.navigationMode === "pan";
      let action;
      if (["arrowleft", "arrowright", "arrowup", "arrowdown"].includes(key)) {
        const x = key === "arrowleft" ? -1 : key === "arrowright" ? 1 : 0,
          y = key === "arrowup" ? 1 : key === "arrowdown" ? -1 : 0;
        action = pan ? { pan: [x, y] } : { orbit: [x * 0.12, y * 0.12] };
      } else if (["w", "a", "s", "d"].includes(key))
        action = {
          pan: [
            key === "a" ? -1 : key === "d" ? 1 : 0,
            key === "w" ? 1 : key === "s" ? -1 : 0,
          ],
        };
      else if (key === "+" || key === "=") action = { zoom: 0.85 };
      else if (key === "-") action = { zoom: 1 / 0.85 };
      else if (key === "f") {
        event.preventDefault();
        event.stopPropagation();
        this.fit();
        return;
      } else if (key === "home" || key === "1" || key === "3" || key === "7") {
        event.preventDefault();
        event.stopPropagation();
        this.viewPreset(
          key === "1"
            ? "front"
            : key === "3"
              ? "right"
              : key === "7"
                ? "top"
                : "iso",
        );
        return;
      } else return;
      event.preventDefault();
      event.stopPropagation();
      this.navigate(action);
    });
    this.ambient = new T.HemisphereLight(
      "#becfe1",
      "#1b2427",
      this.settings.ambient,
    );
    this.scene.add(this.ambient, this.geometry, this.sourceLights);
    this.floor = new T.Mesh(
      new T.PlaneGeometry(100, 100),
      new T.MeshStandardMaterial({
        color: "#354046",
        roughness: 0.88,
        metalness: 0,
      }),
    );
    this.floor.rotation.x = -Math.PI / 2;
    this.floor.position.y = -0.04;
    this.floor.receiveShadow = true;
    this.scene.add(this.floor);
    this.markers = new T.InstancedMesh(
      new T.SphereGeometry(0.055, 8, 6),
      new T.MeshBasicMaterial({ color: 0xffffff }),
      4096,
    );
    this.markers.count = 0;
    this.markers.frustumCulled = false;
    this.layoutMarkers = new T.InstancedMesh(
      new T.TorusGeometry(0.085, 0.012, 5, 14),
      new T.MeshBasicMaterial({ color: 0xffffff, depthTest: false }),
      2048,
    );
    this.layoutMarkers.count = 0;
    this.layoutMarkers.frustumCulled = false;
    this.layoutMarkers.renderOrder = 5;
    this.scene.add(this.markers, this.layoutMarkers);
    this.density = uniform(this.settings.haze);
    this.volumeMaterial = new T.VolumeNodeMaterial();
    this.volumeMaterial.steps = QUALITY[this.options.quality].steps;
    this.volumeMaterial.offsetNode = bayer16(screenCoordinate);
    this.volumeMaterial.scatteringNode = () => this.density;
    this.volume = new T.Mesh(
      new T.BoxGeometry(28, 12, 28),
      this.volumeMaterial,
    );
    this.volume.position.y = 4;
    this.volume.receiveShadow = true;
    this.volume.layers.set(VOLUME);
    this.scene.add(this.volume);
    this.pipeline = new T.RenderPipeline(renderer);
    this.mainPass = pass(this.scene, this.camera);
    this.mainPass.setMRT(
      mrt({
        output,
        diffuseColor,
        normal: packNormalToRGB(normalView),
        velocity,
      }),
    );
    this.mainPass.getTexture("diffuseColor").type = T.UnsignedByteType;
    this.mainPass.getTexture("normal").type = T.UnsignedByteType;
    this.mainColor = this.mainPass.getTextureNode("output");
    const depth = this.mainPass.getTextureNode("depth"),
      normal = this.mainPass.getTextureNode("normal");
    this.gi = ssgi(
      this.mainColor,
      depth,
      sample((uv) => unpackRGBToNormal(normal.sample(uv))),
      this.camera,
    );
    this.gi.giIntensity.value = 1.2;
    this.gi.radius.value = 3;
    this.gi.thickness.value = 0.1;
    this.gi.sliceCount.value = QUALITY[this.options.quality].slices;
    this.gi.stepCount.value = QUALITY[this.options.quality].giSteps;
    this.gi.useTemporalFiltering = true;
    this.volumeMaterial.depthNode = depth.sample(screenUV);
    this.volumePass = pass(this.scene, this.camera, { depthBuffer: false });
    const layers = new T.Layers();
    layers.set(VOLUME);
    this.volumePass.setLayers(layers);
    this.volumePass.setResolutionScale(0.25);
    this.blur = gaussianBlur(this.volumePass, uniform(0.5));
    this.compose();
    this.resize = new ResizeObserver(() => this.setSize());
    this.resize.observe(this.canvas.parentElement);
    this.setSize();
    const pointers = new Map();
    let selection = null,
      multi = false;
    this.listen(this.canvas, "pointerdown", (event) => {
      this.canvas.focus({ preventScroll: true });
      pointers.set(event.pointerId, [event.clientX, event.clientY]);
      if (pointers.size > 1) {
        multi = true;
        selection = null;
      } else if (event.button === 0 && this.navigationMode === "orbit")
        selection = { id: event.pointerId, x: event.clientX, y: event.clientY };
    });
    this.listen(this.canvas, "pointermove", (event) => {
      if (
        selection?.id === event.pointerId &&
        Math.hypot(event.clientX - selection.x, event.clientY - selection.y) > 5
      )
        selection = null;
    });
    this.listen(this.canvas, "pointerup", (event) => {
      const click = selection?.id === event.pointerId && !multi;
      pointers.delete(event.pointerId);
      selection = null;
      if (!pointers.size) multi = false;
      if (!click) return;
      const rect = this.canvas.getBoundingClientRect();
      this.pointer.set(
        ((event.clientX - rect.left) / rect.width) * 2 - 1,
        -((event.clientY - rect.top) / rect.height) * 2 + 1,
      );
      this.raycaster.setFromCamera(this.pointer, this.camera);
      const hit = this.raycaster.intersectObjects([
        this.layoutMarkers,
        this.markers,
      ])[0];
      if (hit) {
        const id = (
          hit.object === this.layoutMarkers ? this.layouts : this.picked
        )[hit.instanceId];
        if (id) {
          this.selected = id;
          this.onSelect(id);
          if (this.data) this.update(this.data);
        }
      }
    });
    this.listen(this.canvas, "pointercancel", (event) => {
      pointers.delete(event.pointerId);
      selection = null;
      if (!pointers.size) multi = false;
    });
    this.listen(this.canvas, "webglcontextlost", (event) => {
      event.preventDefault();
      this.stop();
      this.onError(
        Error("GPU context lost. Reopen the preview to restore it."),
      );
    });
  }
  listen(target, type, listener) {
    target.addEventListener(type, listener);
    this.listeners.push(() => target.removeEventListener(type, listener));
  }
  setSize() {
    if (!this.renderer) return;
    const r = this.canvas.parentElement.getBoundingClientRect(),
      q = QUALITY[this.options.quality],
      scale = Math.min(
        q.ratio,
        1200 / Math.max(1, r.width),
        900 / Math.max(1, r.height),
      );
    this.renderer.setSize(
      Math.max(1, Math.floor(r.width * scale)),
      Math.max(1, Math.floor(r.height * scale)),
      false,
    );
    this.camera.aspect = r.width / Math.max(1, r.height);
    this.camera.updateProjectionMatrix();
  }
  compose() {
    if (!this.pipeline) return;
    let node = this.mainColor;
    if (this.settings.gi)
      node = vec4(
        this.mainColor.rgb
          .mul(this.gi.getAONode())
          .add(
            this.mainPass
              .getTextureNode("diffuseColor")
              .rgb.mul(this.gi.getGINode().rgb),
          ),
        this.mainColor.a,
      );
    if (this.settings.volumes) node = node.add(this.blur);
    this.temporal?.dispose();
    this.temporal = traa(
      node,
      this.mainPass.getTextureNode("depth"),
      this.mainPass.getTextureNode("velocity"),
      this.camera,
    );
    this.pipeline.outputNode = this.temporal;
    this.pipeline.needsUpdate = true;
  }
  configure(settings) {
    const old = this.settings;
    this.settings = { ...old, ...settings };
    this.renderer.toneMappingExposure = this.settings.exposure;
    this.ambient.intensity = this.settings.ambient;
    this.density.value = this.settings.haze;
    this.layoutMarkers.visible = this.settings.layout;
    if (old.gi !== this.settings.gi || old.volumes !== this.settings.volumes)
      this.compose();
    if (this.data) this.update(this.data);
  }
  update(data) {
    const reset =
      this.data &&
      (this.data.blackout !== data.blackout || this.data.name !== data.name);
    this.data = data;
    if (reset) this.compose();
    this.stats.fixtures = data.fixtures.length;
    let pixel = 0;
    this.picked = [];
    this.layouts = [];
    for (const f of data.fixtures) {
      if (this.layouts.length < 2048) {
        const index = this.layouts.length;
        this.matrix.position.fromArray(f.layoutPosition);
        this.matrix.rotation.set(Math.PI / 2, 0, 0);
        this.matrix.scale.setScalar(f.id === this.selected ? 1.5 : 1);
        this.matrix.updateMatrix();
        this.layoutMarkers.setMatrixAt(index, this.matrix.matrix);
        this.color.set(f.id === this.selected ? "#72dec2" : "#637a7c");
        this.layoutMarkers.setColorAt(index, this.color);
        this.layouts.push(f.id);
      }
      for (const p of f.pixels) {
        if (pixel >= 4096) break;
        this.matrix.position.fromArray(p.position);
        this.matrix.rotation.set(0, 0, 0);
        this.matrix.scale.setScalar(f.id === this.selected ? 1.4 : 1);
        this.matrix.updateMatrix();
        this.markers.setMatrixAt(pixel, this.matrix.matrix);
        this.color.setRGB(...p.color.map((v) => Math.max(0.012, v) * 2));
        this.markers.setColorAt(pixel, this.color);
        this.picked.push(f.id);
        pixel++;
      }
    }
    this.markers.count = pixel;
    this.layoutMarkers.count = this.layouts.length;
    this.markers.instanceMatrix.needsUpdate = true;
    this.layoutMarkers.instanceMatrix.needsUpdate = true;
    if (pixel) this.markers.instanceColor.needsUpdate = true;
    if (this.layouts.length)
      this.layoutMarkers.instanceColor.needsUpdate = true;
    this.stats.pixels = pixel;
    const explicit = this.settings.emitters ? data.emitters : [],
      explicitFixtures = new Set(explicit.map((source) => source.fixture));
    const sources = [
      ...explicit,
      ...data.fixtures.filter((f) => !explicitFixtures.has(f.id)),
    ];
    const chosen = sources.slice(0, QUALITY[this.options.quality].lights);
    this.stats.sourceCount = sources.length;
    this.stats.lights = chosen.length;
    const signature = chosen
      .map((source) => source.id + ":" + source.kind)
      .join("|");
    if (signature !== this.lightSignature) {
      for (const object of [...this.sourceLights.children]) {
        this.sourceLights.remove(object);
        object.dispose?.();
      }
      this.lights = [];
      for (const source of chosen) {
        let light;
        if (source.kind === "spot") {
          light = new T.SpotLight(
            0xffffff,
            0,
            source.range,
            (source.angle * Math.PI) / 360,
            0.5,
            2,
          );
          this.sourceLights.add(light.target);
        } else if (source.kind === "plane") {
          if (!this.ltc) {
            RectAreaLightTexturesLib.init();
            T.RectAreaLightNode.setLTC(RectAreaLightTexturesLib);
            this.ltc = true;
          }
          light = new T.RectAreaLight(0xffffff, 0, source.width, source.height);
        } else light = new T.PointLight(0xffffff, 0, source.range, 2);
        light.layers.enable(VOLUME);
        light.castShadow =
          source.kind !== "plane" &&
          this.lights.length < QUALITY[this.options.quality].shadows;
        light.shadow?.mapSize.set(384, 384);
        if (light.shadow) {
          light.shadow.bias = -0.001;
          light.shadow.normalBias = 0.02;
          light.shadow.camera.near = 0.03;
        }
        this.sourceLights.add(light);
        this.lights.push(light);
      }
      this.lightSignature = signature;
    }
    for (let i = 0; i < chosen.length; i++) {
      const source = chosen[i],
        light = this.lights[i],
        peak = Math.max(...source.color, 0);
      light.position.fromArray(source.position);
      light.color.setRGB(...source.color.map((v) => (peak ? v / peak : 0)));
      light.intensity = peak * this.settings.gain;
      light.distance = source.range;
      if (light.isSpotLight) {
        light.angle = (source.angle * Math.PI) / 360;
        light.target.position
          .copy(light.position)
          .add(new T.Vector3().fromArray(source.direction));
      } else if (light.isRectAreaLight) {
        light.width = source.width;
        light.height = source.height;
        light.lookAt(
          new T.Vector3()
            .fromArray(source.position)
            .add(new T.Vector3().fromArray(source.direction)),
        );
      }
    }
  }
  cameraState() {
    return {
      position: this.camera.position.toArray(),
      target: this.controls.target.toArray(),
      up: this.camera.up.toArray(),
    };
  }
  restoreCamera(pose) {
    validateCamera(pose);
    const damping = this.controls.enableDamping;
    this.controls.enableDamping = false;
    this.controls.update();
    this.camera.position.fromArray(pose.position);
    this.camera.up.fromArray(pose.up);
    this.controls.target.fromArray(pose.target);
    this.controls.update();
    this.controls.enableDamping = damping;
    this.cameraChanged();
  }
  navigate(action) {
    this.restoreCamera(navigateCamera(this.cameraState(), action));
  }
  viewPreset(name) {
    this.restoreCamera(cameraPreset(this.cameraState(), name));
  }
  navigationModeSet(mode) {
    this.navigationMode = mode === "pan" ? "pan" : "orbit";
    this.controls.mouseButtons.LEFT =
      this.navigationMode === "pan" ? T.MOUSE.PAN : T.MOUSE.ROTATE;
    this.controls.touches.ONE =
      this.navigationMode === "pan" ? T.TOUCH.PAN : T.TOUCH.ROTATE;
    this.cameraChanged();
  }
  focusSelected() {
    const fixture = this.data?.fixtures.find((f) => f.id === this.selected);
    if (!fixture) {
      this.fit();
      return;
    }
    const pose = this.cameraState(),
      target = fixture.position || fixture.layoutPosition,
      offset = pose.position.map((v, i) => v - pose.target[i]),
      distance = Math.hypot(...offset);
    this.restoreCamera({
      ...pose,
      target: [...target],
      position: target.map((v, i) => v + (offset[i] / distance) * 2.5),
    });
  }
  fit() {
    const bounds = new T.Box3();
    if (this.data)
      for (const fixture of this.data.fixtures)
        bounds.expandByPoint(new T.Vector3().fromArray(fixture.layoutPosition));
    if (this.geometry.children.length)
      bounds.union(new T.Box3().setFromObject(this.geometry));
    if (bounds.isEmpty())
      bounds.setFromCenterAndSize(
        new T.Vector3(0, 1, 0),
        new T.Vector3(4, 2, 4),
      );
    const size = bounds.getSize(new T.Vector3()),
      center = bounds.getCenter(new T.Vector3()),
      radius = Math.max(2, size.length());
    this.camera.up.set(0, 1, 0);
    this.controls.target.copy(center);
    this.camera.position
      .copy(center)
      .add(
        new T.Vector3(0.85, 0.55, -1).normalize().multiplyScalar(radius * 1.2),
      );
    this.camera.far = Math.max(300, radius * 20);
    this.camera.updateProjectionMatrix();
    this.controls.update();
    this.floor.position.y = Math.min(-0.04, bounds.min.y - 0.04);
  }
  async loadGeometry(buffer) {
    const generation = ++this.geometryGeneration;
    if (buffer.byteLength > 32 * 1024 * 1024)
      throw Error("Use a self-contained GLB under 32 MB");
    const manager = new T.LoadingManager();
    manager.setURLModifier((url) => {
      if (url.startsWith("data:") || url.startsWith("blob:")) return url;
      throw Error(
        "External resources are disabled. Import a self-contained GLB.",
      );
    });
    const gltf = await new GLTFLoader(manager).parseAsync(buffer, "");
    if (generation !== this.geometryGeneration) {
      disposeTree(gltf.scene);
      return null;
    }
    const bounds = new T.Box3().setFromObject(gltf.scene);
    if (
      ![...bounds.min, ...bounds.max].every(Number.isFinite) ||
      bounds.getSize(new T.Vector3()).length() > 100000
    ) {
      disposeTree(gltf.scene);
      throw Error("Geometry bounds are invalid or too large");
    }
    this.clearGeometry();
    gltf.scene.traverse((object) => {
      if (object.isLight) object.visible = false;
      if (object.isMesh) {
        object.castShadow = true;
        object.receiveShadow = true;
      }
    });
    this.geometry.add(gltf.scene);
    this.fit();
    return gltf.scene;
  }
  clearGeometry() {
    this.geometryGeneration++;
    for (const child of [...this.geometry.children]) {
      this.geometry.remove(child);
      disposeTree(child);
    }
  }
  start() {
    if (this.active) return;
    this.active = true;
    this.lastTime = 0;
    this.lastMeter = performance.now();
    this.countMeter = 0;
    this.renderer.setAnimationLoop((time) => {
      if (
        !this.active ||
        document.hidden ||
        time - this.lastTime < 1000 / QUALITY[this.options.quality].fps
      )
        return;
      this.lastTime = time;
      try {
        this.controls.update();
        if (this.advanced) this.pipeline.render();
        else this.renderer.render(this.scene, this.camera);
        this.stats.frames++;
        this.countMeter++;
        if (time - this.lastMeter > 1000) {
          this.stats.fps = Math.round(
            (this.countMeter * 1000) / (time - this.lastMeter),
          );
          this.lastMeter = time;
          this.countMeter = 0;
        }
      } catch (error) {
        this.stop();
        this.onError(error);
      }
    });
  }
  stop() {
    this.active = false;
    this.renderer?.setAnimationLoop(null);
  }
  dispose() {
    this.stop();
    this.resize?.disconnect();
    for (const remove of this.listeners) remove();
    this.controls?.dispose();
    for (const light of this.lights || []) light.dispose?.();
    this.clearGeometry();
    this.gi?.dispose();
    this.mainPass?.dispose();
    this.volumePass?.dispose();
    this.blur?.dispose?.();
    this.temporal?.dispose();
    this.pipeline?.dispose();
    if (this.scene) disposeTree(this.scene);
    this.renderer?.dispose();
  }
}
