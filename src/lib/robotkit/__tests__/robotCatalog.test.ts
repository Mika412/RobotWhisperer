import { afterEach, describe, expect, it, vi } from "vitest";

const manifest = [
  { name: "Iiwa14", directory: "iiwa14", urdf: "iiwa14.urdf" },
  { name: "Ur10e", directory: "ur10e", urdf: "ur10e.urdf" },
  { name: "New Robot", directory: "new_robot", urdf: "new_robot.urdf" },
];

const config = {
  materialPresets: { plastic_glossy: { metalness: 0, roughness: 0.28 } },
  robots: {
    iiwa14: {
      displayName: "KUKA LBR iiwa 14",
      brand: "KUKA",
      orientation: [-90, 0, 180],
      materials: { default: "plastic_glossy" },
    },
    ur10e: { brand: "Universal Robots" },
  },
};

function stubFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string) =>
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve(url.endsWith("manifest.json") ? manifest : config),
      }),
    ),
  );
}

async function freshCatalog() {
  vi.resetModules();
  stubFetch();
  const module = await import("../robotCatalog");
  return { module, catalog: await module.loadRobotCatalog() };
}

afterEach(() => vi.unstubAllGlobals());

describe("loadRobotCatalog", () => {
  it("exposes the material presets from the config", async () => {
    const { catalog } = await freshCatalog();
    expect(catalog.presets.plastic_glossy).toEqual({ metalness: 0, roughness: 0.28 });
  });

  it("prefers the config display name and converts orientation to radians", async () => {
    const { module, catalog } = await freshCatalog();
    const iiwa = module.robotByDirectory(catalog, "iiwa14");
    expect(iiwa?.displayName).toBe("KUKA LBR iiwa 14");
    expect(iiwa?.brand).toBe("KUKA");
    expect(iiwa?.orientation).toEqual([-Math.PI / 2, 0, Math.PI]);
    expect(iiwa?.materials?.default).toBe("plastic_glossy");
  });

  it("falls back to the humanized manifest name and default orientation", async () => {
    const { module, catalog } = await freshCatalog();
    const ur10e = module.robotByDirectory(catalog, "ur10e");
    expect(ur10e?.displayName).toBe("Ur10e");
    expect(ur10e?.brand).toBe("Universal Robots");
    expect(ur10e?.orientation).toEqual([-Math.PI / 2, 0, 0]);
  });

  it("includes folders that have no config entry with sensible defaults", async () => {
    const { module, catalog } = await freshCatalog();
    const robot = module.robotByDirectory(catalog, "new_robot");
    expect(robot?.displayName).toBe("New Robot");
    expect(robot?.brand).toBe("");
    expect(robot?.materials).toBeUndefined();
  });

  it("returns undefined for an unknown directory", async () => {
    const { module, catalog } = await freshCatalog();
    expect(module.robotByDirectory(catalog, "missing")).toBeUndefined();
  });
});
