import { FrameKind } from "$lib/workers/decoderCore";
import { registerVisualizer } from "./registry";
import ImageView from "./views/ImageView.svelte";
import PointCloudView from "./views/PointCloudView.svelte";
import PointCloudVizSettings from "./settings/PointCloudVizSettings.svelte";

let registered = false;

export function registerBuiltinVisualizers(): void {
  if (registered) return;
  registered = true;

  registerVisualizer({
    id: "rw.viz.image",
    displayName: "Image",
    defaultConfig: {},
    component: ImageView,
    accepts: (schema, frameKind) => {
      if (frameKind === FrameKind.Image) return 100;
      const leaf = schema.split(/[/.]/).pop() ?? "";
      return /^(Compressed|Raw)?Image$/.test(leaf) ? 90 : 0;
    },
  });

  registerVisualizer<{ pointSize: number }>({
    id: "rw.viz.pointcloud",
    displayName: "Point Cloud",
    defaultConfig: { pointSize: 0.04 },
    component: PointCloudView,
    settingsComponent: PointCloudVizSettings,
    accepts: (schema, frameKind) =>
      frameKind === FrameKind.PointCloud || /MarkerArray$/.test(schema) ? 100 : 0,
  });
}
