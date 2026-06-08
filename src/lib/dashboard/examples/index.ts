import { registerPane } from "../registry/paneRegistry";
import PointInspector from "./PointInspector.svelte";
import PointInspectorSettings from "./PointInspectorSettings.svelte";

let registered = false;

export function registerExamplePanes(): void {
  if (registered) return;
  registered = true;

  registerPane<{ connectionId: number | null; topic: string; field: string; service: string }>({
    type: "rw.example.point-inspector",
    displayName: "Point Inspector",
    description: "Example custom pane: read one numeric field and call a service.",
    category: "data",
    defaultConfig: { connectionId: null, topic: "", field: "", service: "" },
    component: PointInspector,
    settingsComponent: PointInspectorSettings,
  });
}
