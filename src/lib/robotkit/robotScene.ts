import * as THREE from "three";
import { OrbitCameraController } from "$lib/visualizers/three/OrbitCameraController";
import { environmentTexture } from "./envMap";
import { createFloor } from "./floor";

export interface RobotScene {
  readonly canvas: HTMLCanvasElement;
  readonly renderer: THREE.WebGLRenderer;
  readonly scene: THREE.Scene;
  readonly camera: THREE.PerspectiveCamera;
  readonly controller: OrbitCameraController;
  readonly environment: THREE.Texture;
  home: { center: THREE.Vector3; distance: number };
  resetView(): void;
  requestRender(): void;
  dispose(): void;
}

export function createRobotScene(): RobotScene {
  const canvas = document.createElement("canvas");
  canvas.style.display = "block";
  canvas.style.width = "100%";
  canvas.style.height = "100%";

  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.08;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.setClearColor(0x1b202a, 1);

  const scene = new THREE.Scene();
  const environment = environmentTexture(renderer);
  scene.environment = environment;

  const camera = new THREE.PerspectiveCamera(40, 1, 0.01, 100);
  let needsRender = true;
  const controller = new OrbitCameraController(camera, canvas, {
    distance: 1.6,
    groundLevel: 0,
    onChange: () => {
      needsRender = true;
    },
  });

  scene.add(new THREE.HemisphereLight(0xcdd9ff, 0x0c0e12, 0.32));
  const keyLight = new THREE.DirectionalLight(0xfff4e6, 2.6);
  keyLight.position.set(1.6, 3, 1.4);
  keyLight.castShadow = true;
  keyLight.shadow.mapSize.set(2048, 2048);
  keyLight.shadow.camera.near = 0.1;
  keyLight.shadow.camera.far = 12;
  keyLight.shadow.camera.left = -1.2;
  keyLight.shadow.camera.right = 1.2;
  keyLight.shadow.camera.top = 1.2;
  keyLight.shadow.camera.bottom = -1.2;
  keyLight.shadow.bias = -0.0006;
  keyLight.shadow.normalBias = 0.02;
  scene.add(keyLight);
  const rimLight = new THREE.DirectionalLight(0x9fc4ff, 0.6);
  rimLight.position.set(-2, 1.5, -2.5);
  scene.add(rimLight);

  scene.add(createFloor());

  const shadowCatcher = new THREE.Mesh(
    new THREE.PlaneGeometry(8, 8),
    new THREE.ShadowMaterial({ opacity: 0.4 }),
  );
  shadowCatcher.rotation.x = -Math.PI / 2;
  shadowCatcher.position.y = 0.001;
  shadowCatcher.receiveShadow = true;
  scene.add(shadowCatcher);

  let raf = 0;
  const state: RobotScene = {
    canvas,
    renderer,
    scene,
    camera,
    controller,
    environment,
    home: { center: new THREE.Vector3(), distance: 1.6 },
    resetView() {
      controller.frame(state.home.center, state.home.distance);
    },
    requestRender() {
      needsRender = true;
    },
    dispose() {
      cancelAnimationFrame(raf);
      controller.dispose();
      scene.traverse((object) => {
        const mesh = object as THREE.Mesh;
        if (mesh.geometry) mesh.geometry.dispose();
        const material = mesh.material as THREE.Material | THREE.Material[] | undefined;
        if (Array.isArray(material)) material.forEach((entry) => entry.dispose());
        else material?.dispose();
      });
      environment.dispose();
      renderer.dispose();
    },
  };

  const loop = () => {
    raf = requestAnimationFrame(loop);
    if (!needsRender || !canvas.isConnected || canvas.offsetParent === null) return;
    needsRender = false;
    renderer.render(scene, camera);
  };
  raf = requestAnimationFrame(loop);
  return state;
}

export function mountRobotCanvas(host: HTMLElement, robotScene: RobotScene) {
  host.appendChild(robotScene.canvas);
  resizeRobotScene(robotScene, host.clientWidth, host.clientHeight);
  const observer = new ResizeObserver(() =>
    resizeRobotScene(robotScene, host.clientWidth, host.clientHeight),
  );
  observer.observe(host);
  return {
    destroy() {
      observer.disconnect();
      if (robotScene.canvas.parentElement === host) host.removeChild(robotScene.canvas);
    },
  };
}

export function resizeRobotScene(robotScene: RobotScene, width: number, height: number): void {
  if (width < 2 || height < 2) return;
  robotScene.renderer.setSize(width, height, false);
  robotScene.camera.aspect = width / height;
  robotScene.camera.updateProjectionMatrix();
  robotScene.requestRender();
}

const FRAMING_MARGIN = 1.15;

export function placeModel(robotScene: RobotScene, object: THREE.Object3D): void {
  object.updateMatrixWorld(true);
  let box = new THREE.Box3().setFromObject(object);
  if (box.isEmpty()) return;

  object.position.y -= box.min.y;
  object.updateMatrixWorld(true);
  box = new THREE.Box3().setFromObject(object);

  const center = box.getCenter(new THREE.Vector3());
  const radius = box.getSize(new THREE.Vector3()).length() * 0.5 || 0.3;
  const fov = (robotScene.camera.fov * Math.PI) / 180;
  const distance = (radius / Math.sin(fov / 2)) * FRAMING_MARGIN;
  robotScene.home = { center, distance };
  robotScene.controller.setDistanceLimits(radius * 0.4, distance * 4);
  robotScene.resetView();
}
