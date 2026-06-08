const MESH_SOURCE_EXTENSION = /\.(dae|obj|stl)$/i;

export function toMeshAssetUrl(url: string): string {
  return url.replace(MESH_SOURCE_EXTENSION, ".glb");
}
