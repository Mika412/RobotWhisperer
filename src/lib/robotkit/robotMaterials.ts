import * as THREE from "three";
import type { MaterialOverride, MaterialPreset, RobotMaterialConfig } from "./robotCatalog";

interface ResolvedMaterial {
  metalness: number;
  roughness: number;
  clearcoat: number;
  clearcoatRoughness: number;
  envMapIntensity: number;
  color: THREE.Color | null;
}

const AUTO_STRATEGY = "auto";

const NEUTRAL: Omit<ResolvedMaterial, "color"> = {
  metalness: 0,
  roughness: 0.5,
  clearcoat: 0,
  clearcoatRoughness: 0,
  envMapIntensity: 1,
};

const SATIN_PLASTIC: Omit<ResolvedMaterial, "color"> = {
  metalness: 0,
  roughness: 0.44,
  clearcoat: 0.4,
  clearcoatRoughness: 0.26,
  envMapIntensity: 1,
};

function bakedColor(material: THREE.Material): THREE.Color {
  const color = (material as { color?: THREE.Color }).color;
  return color instanceof THREE.Color ? color.clone() : new THREE.Color(0x9298a0);
}

function luminance(color: THREE.Color): number {
  return 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
}

function autoFinish(baked: THREE.Color): MaterialPreset {
  const value = luminance(baked);
  if (value >= 0.58) return { metalness: 1, roughness: 0.19, envMapIntensity: 1.5 };
  if (value >= 0.3) return { metalness: 1, roughness: 0.4, envMapIntensity: 1.3 };
  return {
    metalness: 0.05,
    roughness: 0.46,
    clearcoat: 0.5,
    clearcoatRoughness: 0.2,
    envMapIntensity: 0.95,
  };
}

function presetByName(
  name: string,
  presets: Record<string, MaterialPreset>,
  baked: THREE.Color,
): MaterialPreset {
  return name === AUTO_STRATEGY ? autoFinish(baked) : (presets[name] ?? {});
}

function specFor(
  override: string | MaterialOverride,
  presets: Record<string, MaterialPreset>,
  baked: THREE.Color,
): MaterialPreset {
  if (typeof override === "string") return presetByName(override, presets, baked);
  const base = override.preset ? presetByName(override.preset, presets, baked) : {};
  return { ...base, ...override };
}

function applyDefaults(spec: MaterialPreset, baked: THREE.Color): ResolvedMaterial {
  return {
    metalness: spec.metalness ?? NEUTRAL.metalness,
    roughness: spec.roughness ?? NEUTRAL.roughness,
    clearcoat: spec.clearcoat ?? NEUTRAL.clearcoat,
    clearcoatRoughness: spec.clearcoatRoughness ?? NEUTRAL.clearcoatRoughness,
    envMapIntensity: spec.envMapIntensity ?? NEUTRAL.envMapIntensity,
    color: spec.color ? new THREE.Color(spec.color) : baked,
  };
}

function nearestLinkName(mesh: THREE.Object3D): string | null {
  let node: THREE.Object3D | null = mesh;
  while (node) {
    if ((node as { isURDFLink?: boolean }).isURDFLink && node.name) return node.name;
    node = node.parent;
  }
  return null;
}

function overrideFor(
  mesh: THREE.Mesh,
  source: THREE.Material,
  config: RobotMaterialConfig | undefined,
): string | MaterialOverride | null {
  if (!config) return null;
  const materialName = source.name;
  if (materialName && config.byMaterial?.[materialName]) return config.byMaterial[materialName];
  const linkName = nearestLinkName(mesh);
  if (linkName && config.byLink?.[linkName]) return config.byLink[linkName];
  return config.default ?? null;
}

function resolveMaterial(
  mesh: THREE.Mesh,
  source: THREE.Material,
  config: RobotMaterialConfig | undefined,
  presets: Record<string, MaterialPreset>,
): ResolvedMaterial {
  const baked = bakedColor(source);
  const override = overrideFor(mesh, source, config);
  if (!override) return { ...SATIN_PLASTIC, color: baked };
  return applyDefaults(specFor(override, presets, baked), baked);
}

function toStandardMaterial(
  mesh: THREE.Mesh,
  source: THREE.Material,
  config: RobotMaterialConfig | undefined,
  presets: Record<string, MaterialPreset>,
): THREE.MeshStandardMaterial {
  const resolved = resolveMaterial(mesh, source, config, presets);
  const textured = source as { map?: THREE.Texture | null; normalMap?: THREE.Texture | null };
  const parameters: THREE.MeshStandardMaterialParameters = {
    metalness: resolved.metalness,
    roughness: resolved.roughness,
    envMapIntensity: resolved.envMapIntensity,
  };
  if (resolved.color) parameters.color = resolved.color;

  const material =
    resolved.clearcoat > 0
      ? new THREE.MeshPhysicalMaterial({
          ...parameters,
          clearcoat: resolved.clearcoat,
          clearcoatRoughness: resolved.clearcoatRoughness,
        })
      : new THREE.MeshStandardMaterial(parameters);

  if (textured.map) material.map = textured.map;
  if (textured.normalMap) material.normalMap = textured.normalMap;
  return material;
}

export function applyRobotMaterials(
  object: THREE.Object3D,
  config: RobotMaterialConfig | undefined,
  presets: Record<string, MaterialPreset>,
): void {
  object.traverse((child) => {
    const mesh = child as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.material = Array.isArray(mesh.material)
      ? mesh.material.map((entry) => toStandardMaterial(mesh, entry, config, presets))
      : toStandardMaterial(mesh, mesh.material, config, presets);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
  });
}
