import { registerPane } from "$lib/dashboard/registry/paneRegistry";
import RobotPane from "./RobotPane.svelte";
import RobotPaneSettings from "./RobotPaneSettings.svelte";

export interface RobotPaneConfig {
  model: string | null;
  jointValues: Record<string, number>;
  showAxes: boolean;
  controlsCollapsed: boolean;
}

let registered = false;

export function registerRobotPanes(): void {
  if (registered) return;
  registered = true;

  registerPane<RobotPaneConfig>({
    type: "rw.robot.model",
    displayName: "Robot Model",
    description: "Pose a photorealistic 3D robot with manual joint controls.",
    category: "visualization",
    group: "Robots",
    defaultConfig: { model: "iiwa14", jointValues: {}, showAxes: false, controlsCollapsed: false },
    component: RobotPane,
    settingsComponent: RobotPaneSettings,
  });
}
