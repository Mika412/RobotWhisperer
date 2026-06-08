export interface MaterialPreset {
  metalness?: number;
  roughness?: number;
  clearcoat?: number;
  clearcoatRoughness?: number;
  envMapIntensity?: number;
  color?: string;
}

export interface MaterialOverride extends MaterialPreset {
  preset?: string;
}

export interface RobotMaterialConfig {
  default?: string | MaterialOverride;
  byMaterial?: Record<string, string | MaterialOverride>;
  byLink?: Record<string, string | MaterialOverride>;
}

export interface RobotDefinition {
  directory: string;
  urdf: string;
  displayName: string;
  brand: string;
  orientation: [number, number, number];
  materials?: RobotMaterialConfig;
}

export interface RobotCatalog {
  presets: Record<string, MaterialPreset>;
  robots: RobotDefinition[];
}

interface DiscoveryEntry {
  name: string;
  directory: string;
  urdf: string;
}

interface RobotMetadata {
  displayName?: string;
  brand?: string;
  orientation?: [number, number, number];
  materials?: RobotMaterialConfig;
}

interface RobotConfigFile {
  materialPresets?: Record<string, MaterialPreset>;
  robots?: Record<string, RobotMetadata>;
}

const DEFAULT_ORIENTATION_DEGREES: [number, number, number] = [-90, 0, 0];

function assetsBase(): string {
  return `${import.meta.env.BASE_URL ?? "/"}assets/`;
}

function toRadians(degrees: [number, number, number]): [number, number, number] {
  const factor = Math.PI / 180;
  return [degrees[0] * factor, degrees[1] * factor, degrees[2] * factor];
}

async function fetchJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const response = await fetch(url, { cache: "no-store" });
    return response.ok ? ((await response.json()) as T) : fallback;
  } catch {
    return fallback;
  }
}

function mergeDefinition(
  entry: DiscoveryEntry,
  metadata: RobotMetadata | undefined,
): RobotDefinition {
  return {
    directory: entry.directory,
    urdf: entry.urdf,
    displayName: metadata?.displayName ?? entry.name,
    brand: metadata?.brand ?? "",
    orientation: toRadians(metadata?.orientation ?? DEFAULT_ORIENTATION_DEGREES),
    materials: metadata?.materials,
  };
}

let catalogPromise: Promise<RobotCatalog> | null = null;

export function loadRobotCatalog(): Promise<RobotCatalog> {
  if (!catalogPromise) {
    catalogPromise = Promise.all([
      fetchJson<DiscoveryEntry[]>(`${assetsBase()}manifest.json`, []),
      fetchJson<RobotConfigFile>(`${assetsBase()}robots.config.json`, {}),
    ]).then(([entries, config]) => ({
      presets: config.materialPresets ?? {},
      robots: entries.map((entry) => mergeDefinition(entry, config.robots?.[entry.directory])),
    }));
  }
  return catalogPromise;
}

export function robotByDirectory(
  catalog: RobotCatalog,
  directory: string,
): RobotDefinition | undefined {
  return catalog.robots.find((robot) => robot.directory === directory);
}

export function entryBaseUrl(definition: RobotDefinition): string {
  return `${assetsBase()}${definition.directory}`;
}

export function entryUrdfUrl(definition: RobotDefinition): string {
  return `${entryBaseUrl(definition)}/${definition.urdf}`;
}
