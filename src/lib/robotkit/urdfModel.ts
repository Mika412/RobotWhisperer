import * as THREE from "three";
import URDFLoader, { type URDFRobot } from "urdf-loader";
import { placeModel, type RobotScene } from "./robotScene";
import { applyRobotMaterials } from "./robotMaterials";
import { loadCollada, loadObj, loadStl } from "./meshLoader";
import {
  entryBaseUrl,
  entryUrdfUrl,
  type MaterialPreset,
  type RobotDefinition,
} from "./robotCatalog";

export function loadRobotModel(
  robotScene: RobotScene,
  definition: RobotDefinition,
  presets: Record<string, MaterialPreset>,
): Promise<URDFRobot> {
  const loader = new URDFLoader();
  loader.packages = { [definition.directory]: entryBaseUrl(definition) };
  loader.fetchOptions = { cache: "no-store" };

  let pendingMeshes = 0;
  let robot: URDFRobot | null = null;
  let parsed = false;
  const reframeWhenSettled = () => {
    if (!parsed || pendingMeshes !== 0 || !robot) return;
    applyRobotMaterials(robot, definition.materials, presets);
    placeModel(robotScene, robot);
  };

  loader.loadMeshCb = (path, _manager, onLoad) => {
    pendingMeshes += 1;
    const complete = (object: THREE.Object3D) => {
      onLoad(object);
      pendingMeshes -= 1;
      robotScene.requestRender();
      reframeWhenSettled();
    };
    const fail = () => complete(new THREE.Object3D());
    if (/\.dae$/i.test(path)) {
      loadCollada(path)
        .then((scene) => complete(scene.clone(true)))
        .catch(fail);
    } else if (/\.stl$/i.test(path)) {
      loadStl(path)
        .then((geometry) => complete(new THREE.Mesh(geometry)))
        .catch(fail);
    } else if (/\.obj$/i.test(path)) {
      loadObj(path)
        .then((geometry) => complete(new THREE.Mesh(geometry)))
        .catch(fail);
    } else {
      fail();
    }
  };

  return new Promise<URDFRobot>((resolve, reject) => {
    loader.load(
      entryUrdfUrl(definition),
      (loaded) => {
        const [rotationX, rotationY, rotationZ] = definition.orientation;
        loaded.rotation.set(rotationX, rotationY, rotationZ);
        robotScene.scene.add(loaded);
        robot = loaded;
        parsed = true;
        placeModel(robotScene, loaded);
        robotScene.requestRender();
        reframeWhenSettled();
        resolve(loaded);
      },
      undefined,
      reject,
    );
  });
}

export function removeRobotModel(robotScene: RobotScene, robot: URDFRobot): void {
  robotScene.scene.remove(robot);
  robot.traverse((object) => {
    const mesh = object as THREE.Mesh;
    if (mesh.geometry) mesh.geometry.dispose();
    const material = mesh.material as THREE.Material | THREE.Material[] | undefined;
    if (Array.isArray(material)) material.forEach((entry) => entry.dispose());
    else material?.dispose();
  });
}
