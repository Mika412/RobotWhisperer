import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { applyRobotMaterials } from "../robotMaterials";
import type { MaterialPreset, RobotMaterialConfig } from "../robotCatalog";

const presets: Record<string, MaterialPreset> = {
  plastic_glossy: { metalness: 0, roughness: 0.28, clearcoat: 0.9, clearcoatRoughness: 0.18 },
  metal_brushed: { metalness: 0.92, roughness: 0.42, envMapIntensity: 1.25 },
};

function meshWithMaterialNamed(name: string, color: number): THREE.Mesh {
  const material = new THREE.MeshStandardMaterial({ color });
  material.name = name;
  return new THREE.Mesh(new THREE.BufferGeometry(), material);
}

function linkContaining(name: string, mesh: THREE.Mesh): THREE.Object3D {
  const link = new THREE.Object3D() as THREE.Object3D & { isURDFLink: boolean };
  link.isURDFLink = true;
  link.name = name;
  link.add(mesh);
  return link;
}

describe("applyRobotMaterials", () => {
  it("resolves a named material override with a colour to a clearcoat physical material", () => {
    const mesh = meshWithMaterialNamed("Orange", 0xff6a0a);
    const config: RobotMaterialConfig = {
      byMaterial: { Orange: { preset: "plastic_glossy", color: "#1e63ff" } },
    };

    applyRobotMaterials(mesh, config, presets);

    const material = mesh.material as THREE.MeshPhysicalMaterial;
    expect(material).toBeInstanceOf(THREE.MeshPhysicalMaterial);
    expect(material.clearcoat).toBe(0.9);
    expect(material.color.getHexString()).toBe("1e63ff");
  });

  it("prefers a material override over a link override over the default", () => {
    const mesh = meshWithMaterialNamed("Orange", 0xffffff);
    linkContaining("link_3", mesh);
    const config: RobotMaterialConfig = {
      default: "metal_brushed",
      byLink: { link_3: "metal_brushed" },
      byMaterial: { Orange: "plastic_glossy" },
    };

    applyRobotMaterials(mesh, config, presets);

    const material = mesh.material as THREE.MeshPhysicalMaterial;
    expect(material.roughness).toBeCloseTo(0.28);
  });

  it("falls back to a link override when the material name is unmatched", () => {
    const mesh = meshWithMaterialNamed("Unmapped", 0xffffff);
    linkContaining("wrist", mesh);
    const config: RobotMaterialConfig = { byLink: { wrist: "metal_brushed" } };

    applyRobotMaterials(mesh, config, presets);

    const material = mesh.material as THREE.MeshStandardMaterial;
    expect(material.metalness).toBeCloseTo(0.92);
  });

  it("preserves the baked colour and maps when no config matches", () => {
    const map = new THREE.Texture();
    const source = new THREE.MeshStandardMaterial({ color: 0x3366cc });
    source.map = map;
    const mesh = new THREE.Mesh(new THREE.BufferGeometry(), source);

    applyRobotMaterials(mesh, undefined, presets);

    const material = mesh.material as THREE.MeshStandardMaterial;
    expect(material).toBeInstanceOf(THREE.MeshStandardMaterial);
    expect(material.color.getHexString()).toBe("3366cc");
    expect(material.map).toBe(map);
    expect(material.metalness).toBe(0);
  });

  it("merges inline overrides on top of the referenced preset", () => {
    const mesh = meshWithMaterialNamed("Grey", 0x808080);
    const config: RobotMaterialConfig = {
      byMaterial: { Grey: { preset: "metal_brushed", roughness: 0.1 } },
    };

    applyRobotMaterials(mesh, config, presets);

    const material = mesh.material as THREE.MeshStandardMaterial;
    expect(material.metalness).toBeCloseTo(0.92);
    expect(material.roughness).toBeCloseTo(0.1);
  });

  it("classifies a bright surface as polished metal under the auto strategy", () => {
    const mesh = meshWithMaterialNamed("shell", 0xe8e8e8);
    applyRobotMaterials(mesh, { default: "auto" }, presets);

    const material = mesh.material as THREE.MeshStandardMaterial;
    expect(material.metalness).toBe(1);
    expect(material.roughness).toBeLessThan(0.25);
    expect(material.color.getHexString()).toBe("e8e8e8");
  });

  it("classifies a dark surface as plastic under the auto strategy", () => {
    const mesh = meshWithMaterialNamed("base", 0x161616);
    applyRobotMaterials(mesh, { default: "auto" }, presets);

    const material = mesh.material as THREE.MeshStandardMaterial;
    expect(material.metalness).toBeLessThan(0.2);
  });

  it("enables shadows on every mesh", () => {
    const mesh = meshWithMaterialNamed("White", 0xffffff);
    applyRobotMaterials(mesh, undefined, presets);
    expect(mesh.castShadow).toBe(true);
    expect(mesh.receiveShadow).toBe(true);
  });
});
