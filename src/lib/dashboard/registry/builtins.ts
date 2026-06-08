import { registerPane } from "$lib/dashboard/registry/paneRegistry";
import { registerBuiltinVisualizers } from "$lib/visualizers/builtins";
import PlaceholderPane from "$lib/dashboard/panes/PlaceholderPane.svelte";
import RawPane from "$lib/dashboard/panes/RawPane.svelte";
import PlotPane from "$lib/dashboard/panes/PlotPane.svelte";
import ImagePane from "$lib/dashboard/panes/ImagePane.svelte";
import PointCloudPane from "$lib/dashboard/panes/PointCloudPane.svelte";
import PlotVizSettings from "$lib/visualizers/settings/PlotVizSettings.svelte";
import PointCloudVizSettings from "$lib/visualizers/settings/PointCloudVizSettings.svelte";

let registered = false;

export function registerBuiltinPanes(): void {
  if (registered) return;
  registered = true;

  registerBuiltinVisualizers();

  registerPane({
    type: "rw.placeholder",
    displayName: "Empty pane",
    category: "misc",
    defaultConfig: {},
    component: PlaceholderPane,
  });

  registerPane<{ connectionId: number | null; topic: string }>({
    type: "rw.raw",
    displayName: "Raw / JSON",
    description: "Stream the latest decoded value of a topic as formatted JSON.",
    category: "data",
    defaultConfig: { connectionId: null, topic: "" },
    component: RawPane,
  });

  registerPane<{
    connectionId: number | null;
    topic: string;
    windowSeconds: number;
    series: { id: string; path: string; label: string; color: string }[];
  }>({
    type: "rw.plot",
    displayName: "Plot",
    description: "Stream numeric field values from a topic as time-series lines.",
    category: "visualization",
    defaultConfig: { connectionId: null, topic: "", windowSeconds: 10, series: [] },
    component: PlotPane,
    settingsComponent: PlotVizSettings,
  });

  registerPane<{ connectionId: number | null; topic: string }>({
    type: "rw.image",
    displayName: "Image",
    description: "Render a sensor_msgs/Image topic as a live canvas.",
    category: "visualization",
    defaultConfig: { connectionId: null, topic: "" },
    component: ImagePane,
  });

  registerPane<{ connectionId: number | null; topic: string; pointSize: number }>({
    type: "rw.pointcloud",
    displayName: "Point Cloud",
    description: "Render a visualization_msgs/MarkerArray point set in 3D.",
    category: "visualization",
    defaultConfig: { connectionId: null, topic: "", pointSize: 0.04 },
    component: PointCloudPane,
    settingsComponent: PointCloudVizSettings,
  });
}

export const DEFAULT_PANE_TYPE = "rw.placeholder";
