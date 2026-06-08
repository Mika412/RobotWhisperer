import type { URDFRobot } from "urdf-loader";

export interface JointDriver {
  setJoint(name: string, value: number): void;
  setJoints(values: Record<string, number>): void;
  applyNamedPositions(names: string[], positions: number[]): void;
  current(name: string): number;
}

export function createJointDriver(robot: URDFRobot, onChange: () => void): JointDriver {
  return {
    setJoint(name, value) {
      const joint = robot.joints[name];
      if (!joint) return;
      joint.setJointValue(value);
      onChange();
    },
    setJoints(values) {
      let changed = false;
      for (const [name, value] of Object.entries(values)) {
        const joint = robot.joints[name];
        if (!joint) continue;
        joint.setJointValue(value);
        changed = true;
      }
      if (changed) onChange();
    },
    applyNamedPositions(names, positions) {
      const count = Math.min(names.length, positions.length);
      let changed = false;
      for (let index = 0; index < count; index += 1) {
        const joint = robot.joints[names[index]];
        if (!joint) continue;
        joint.setJointValue(positions[index]);
        changed = true;
      }
      if (changed) onChange();
    },
    current(name) {
      return robot.joints[name]?.angle ?? 0;
    },
  };
}
