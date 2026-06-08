import * as THREE from "three";
import { ColladaLoader } from "three/examples/jsm/loaders/ColladaLoader.js";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { OBJLoader } from "three/examples/jsm/loaders/OBJLoader.js";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";

const colladaCache = new Map<string, Promise<THREE.Object3D>>();
const stlCache = new Map<string, Promise<THREE.BufferGeometry>>();
const objCache = new Map<string, Promise<THREE.BufferGeometry>>();

let colladaLoader: ColladaLoader | null = null;
let stlLoader: STLLoader | null = null;
let objLoader: OBJLoader | null = null;

export function loadCollada(url: string): Promise<THREE.Object3D> {
  let pending = colladaCache.get(url);
  if (!pending) {
    colladaLoader ??= new ColladaLoader();
    pending = colladaLoader.loadAsync(url).then((collada) => {
      const scene = (collada as { scene?: THREE.Object3D } | null)?.scene;
      if (!scene) throw new Error(`empty collada: ${url}`);
      return scene;
    });
    colladaCache.set(url, pending);
  }
  return pending;
}

export function loadStl(url: string): Promise<THREE.BufferGeometry> {
  let pending = stlCache.get(url);
  if (!pending) {
    stlLoader ??= new STLLoader();
    pending = stlLoader.loadAsync(url);
    stlCache.set(url, pending);
  }
  return pending;
}

export function loadObj(url: string): Promise<THREE.BufferGeometry> {
  let pending = objCache.get(url);
  if (!pending) {
    objLoader ??= new OBJLoader();
    pending = objLoader.loadAsync(url).then((group) => {
      group.updateMatrixWorld(true);
      const geometries: THREE.BufferGeometry[] = [];
      group.traverse((child) => {
        const mesh = child as THREE.Mesh;
        if (!mesh.isMesh) return;
        const geometry = mesh.geometry.clone();
        geometry.applyMatrix4(mesh.matrixWorld);
        if (!geometry.getAttribute("normal")) geometry.computeVertexNormals();
        for (const attribute of ["uv", "uv1", "uv2"]) geometry.deleteAttribute(attribute);
        geometries.push(geometry);
      });
      if (geometries.length === 0) throw new Error(`empty obj: ${url}`);
      const merged = geometries.length === 1 ? geometries[0] : mergeGeometries(geometries, false);
      if (!merged) throw new Error(`merge failed: ${url}`);
      return merged;
    });
    objCache.set(url, pending);
  }
  return pending;
}
