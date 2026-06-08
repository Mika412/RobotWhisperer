<script lang="ts">
  import { untrack } from "svelte";
  import * as THREE from "three";
  import { Focus } from "@lucide/svelte";
  import type { URDFRobot } from "urdf-loader";
  import type { PaneComponentProps } from "$lib/dashboard/registry/paneRegistry";
  import {
    createRobotScene,
    mountRobotCanvas,
    loadRobotCatalog,
    robotByDirectory,
    loadRobotModel,
    removeRobotModel,
    createJointDriver,
    clampToJoint,
    describeJoints,
    type RobotScene,
    type JointDriver,
    type JointHandle,
  } from "$lib/robotkit";
  import JointControlsOverlay from "./controls/JointControlsOverlay.svelte";
  import RobotInfoOverlay from "./controls/RobotInfoOverlay.svelte";

  interface RobotPaneConfig {
    model: string | null;
    jointValues: Record<string, number>;
    showAxes: boolean;
    controlsCollapsed: boolean;
  }

  let { config, ctx }: PaneComponentProps<RobotPaneConfig> = $props();

  let host = $state<HTMLDivElement>();
  let sceneReady = $state(false);
  let status = $state<"empty" | "loading" | "ready" | "error">("empty");
  let handles = $state<JointHandle[]>([]);
  let poseValues = $state<Record<string, number>>({});
  let modelLabel = $state("");
  let modelBrand = $state("");

  let robotScene: RobotScene | null = null;
  let axes: THREE.AxesHelper | null = null;
  let robot: URDFRobot | null = null;
  let driver: JointDriver | null = null;
  let loadedModel: string | null = null;
  let loadToken = 0;
  let persistTimer: ReturnType<typeof setTimeout> | null = null;

  function schedulePersist() {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      persistTimer = null;
      ctx.persist({ jointValues: poseValues });
    }, 400);
  }

  function flushPersist() {
    if (!persistTimer) return;
    clearTimeout(persistTimer);
    persistTimer = null;
    ctx.persist({ jointValues: poseValues });
  }

  function adoptRobot(loaded: URDFRobot, savedPose: Record<string, number>) {
    robot = loaded;
    driver = createJointDriver(loaded, () => robotScene?.requestRender());
    handles = describeJoints(loaded);
    const pose: Record<string, number> = {};
    for (const handle of handles)
      pose[handle.name] = clampToJoint(handle, savedPose[handle.name] ?? 0);
    driver.setJoints(pose);
    poseValues = pose;
  }

  function setJoint(name: string, value: number) {
    poseValues = { ...poseValues, [name]: value };
    driver?.setJoint(name, value);
    schedulePersist();
  }

  function resetPose() {
    const pose: Record<string, number> = {};
    for (const handle of handles) pose[handle.name] = clampToJoint(handle, 0);
    driver?.setJoints(pose);
    poseValues = pose;
    schedulePersist();
  }

  function resetView() {
    robotScene?.resetView();
  }

  $effect(() => {
    if (!host) return;
    const scene = createRobotScene();
    robotScene = scene;
    axes = new THREE.AxesHelper(0.3);
    axes.visible = untrack(() => config.showAxes);
    scene.scene.add(axes);
    const mount = mountRobotCanvas(host, scene);
    sceneReady = true;

    return () => {
      flushPersist();
      sceneReady = false;
      loadToken += 1;
      loadedModel = null;
      mount.destroy();
      scene.dispose();
      robotScene = null;
      robot = null;
      driver = null;
      axes = null;
    };
  });

  $effect(() => {
    if (axes) axes.visible = config.showAxes;
    robotScene?.requestRender();
  });

  $effect(() => {
    const modelId = config.model;
    if (!sceneReady || !robotScene) return;
    if (modelId === loadedModel) return;
    loadedModel = modelId;
    const scene = robotScene;
    const token = ++loadToken;

    if (robot) {
      removeRobotModel(scene, robot);
      robot = null;
      driver = null;
      handles = [];
    }

    if (!modelId) {
      status = "empty";
      return;
    }

    status = "loading";
    const savedPose = untrack(() => config.jointValues ?? {});
    void loadRobotCatalog()
      .then((catalog) => {
        const definition = robotByDirectory(catalog, modelId);
        if (!definition) throw new Error(`unknown model: ${modelId}`);
        modelLabel = definition.displayName;
        modelBrand = definition.brand;
        return loadRobotModel(scene, definition, catalog.presets);
      })
      .then((loaded) => {
        if (token !== loadToken) {
          removeRobotModel(scene, loaded);
          return;
        }
        adoptRobot(loaded, savedPose);
        status = "ready";
      })
      .catch(() => {
        if (token === loadToken) status = "error";
      });
  });
</script>

<div class="robot" bind:this={host}>
  {#if status === "empty"}
    <p class="hint">Open the pane settings (gear) to choose a robot model.</p>
  {/if}

  {#if status === "ready"}
    <div class="overlay-stack">
      <RobotInfoOverlay model={modelLabel} brand={modelBrand} />
      <JointControlsOverlay
        joints={handles}
        values={poseValues}
        collapsed={config.controlsCollapsed}
        oncollapse={(collapsed) => ctx.persist({ controlsCollapsed: collapsed })}
        onjoint={setJoint}
        onreset={resetPose}
      />
    </div>
    <button
      class="view-reset"
      title="Reset camera view"
      aria-label="Reset camera view"
      onclick={resetView}
    >
      <Focus size={16} />
    </button>
  {/if}

  <div class="badge" class:show={status === "loading" || status === "error"}>
    {#if status === "loading"}
      Loading {modelLabel || "model"}…
    {:else if status === "error"}
      Failed to load {modelLabel || "model"}
    {/if}
  </div>
</div>

<style>
  .robot {
    position: relative;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    background: var(--color-bg-deep, var(--color-bg-main));
  }
  .hint {
    position: absolute;
    inset: 0;
    margin: auto;
    width: max-content;
    height: max-content;
    text-align: center;
    font-size: 12px;
    color: var(--color-text-disabled);
  }
  .overlay-stack {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 244px;
    max-width: calc(100% - 24px);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .view-reset {
    position: absolute;
    bottom: 12px;
    right: 12px;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    color: var(--color-text-main);
    background: color-mix(in srgb, var(--color-bg-main) 70%, transparent);
    backdrop-filter: blur(16px) saturate(1.1);
    -webkit-backdrop-filter: blur(16px) saturate(1.1);
    border: 1px solid color-mix(in srgb, var(--color-text-main) 11%, transparent);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.16);
    cursor: pointer;
    transition:
      color 0.15s ease,
      background 0.15s ease,
      border-color 0.15s ease;
  }
  .view-reset:hover {
    color: var(--color-accent, #6ea8fe);
    background: color-mix(in srgb, var(--color-bg-main) 84%, transparent);
    border-color: color-mix(in srgb, var(--color-accent, #6ea8fe) 35%, transparent);
  }
  .badge {
    position: absolute;
    bottom: 10px;
    left: 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-disabled);
    opacity: 0;
    pointer-events: none;
  }
  .badge.show {
    opacity: 1;
  }
</style>
