import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { DRACOLoader } from "three/examples/jsm/loaders/DRACOLoader.js";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import { toMeshAssetUrl } from "./meshAssetUrl";

const meshCache = new Map<string, Promise<THREE.Object3D>>();

let gltfLoader: GLTFLoader | null = null;

function decoderPath(): string {
  return `${import.meta.env.BASE_URL ?? "/"}draco/`;
}

function sharedLoader(): GLTFLoader {
  if (!gltfLoader) {
    const dracoLoader = new DRACOLoader().setDecoderPath(decoderPath());
    gltfLoader = new GLTFLoader().setDRACOLoader(dracoLoader);
  }
  return gltfLoader;
}

function flattenToMesh(scene: THREE.Object3D): THREE.Object3D {
  scene.updateMatrixWorld(true);
  const geometries: THREE.BufferGeometry[] = [];
  scene.traverse((child) => {
    const mesh = child as THREE.Mesh;
    if (!mesh.isMesh) return;
    const geometry = mesh.geometry.clone();
    geometry.applyMatrix4(mesh.matrixWorld);
    geometries.push(geometry);
  });
  if (geometries.length === 0) return scene;
  const merged = geometries.length === 1 ? geometries[0] : mergeGeometries(geometries, false);
  return merged ? new THREE.Mesh(merged) : scene;
}

export function loadMesh(url: string): Promise<THREE.Object3D> {
  let pending = meshCache.get(url);
  if (!pending) {
    const singleMaterial = /\.(obj|stl)$/i.test(url);
    pending = sharedLoader()
      .loadAsync(toMeshAssetUrl(url))
      .then((gltf) => (singleMaterial ? flattenToMesh(gltf.scene) : gltf.scene));
    meshCache.set(url, pending);
  }
  return pending;
}
