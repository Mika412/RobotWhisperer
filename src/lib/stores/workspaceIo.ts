import { workspaceRpc } from "$lib/core/workspaceRpc";
import { requestsStore } from "$lib/stores/requestsStore.svelte";
import { connectionStore } from "$lib/stores/connectionStore.svelte";
import type { ImportMode, ImportReport, WorkspaceFile } from "$lib/core/types";

export async function downloadWorkspace(): Promise<void> {
  const json = await workspaceRpc.exportWorkspace();
  const url = URL.createObjectURL(new Blob([json], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "robot-whisperer-workspace.json";
  anchor.click();
  URL.revokeObjectURL(url);
}

export async function importWorkspaceFile(
  file: File,
  mode: ImportMode = "merge",
): Promise<ImportReport> {
  const parsed = JSON.parse(await file.text()) as WorkspaceFile;
  const report = await workspaceRpc.importWorkspace(parsed, mode);
  await Promise.all([requestsStore.load(), connectionStore.load()]);
  return report;
}
