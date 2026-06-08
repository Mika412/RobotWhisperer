import type { URDFRobot } from "urdf-loader";

export type ControllableJointType = "revolute" | "continuous" | "prismatic";

export interface JointHandle {
  name: string;
  label: string;
  type: ControllableJointType;
  lower: number;
  upper: number;
  value: number;
}

const CONTINUOUS_RANGE = Math.PI;

function humanizeJoint(name: string): string {
  return name
    .replace(/[_-]+/g, " ")
    .replace(/([a-zA-Z])(\d)/g, "$1 $2")
    .trim()
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function isControllable(type: string): type is ControllableJointType {
  return type === "revolute" || type === "continuous" || type === "prismatic";
}

function isMimic(joint: object): boolean {
  const mimic = (joint as { mimicJoint?: unknown }).mimicJoint;
  return typeof mimic === "string" && mimic.length > 0;
}

export function describeJoints(robot: URDFRobot): JointHandle[] {
  const handles: JointHandle[] = [];
  for (const [name, joint] of Object.entries(robot.joints)) {
    if (!isControllable(joint.jointType) || isMimic(joint)) continue;
    const continuous = joint.jointType === "continuous";
    handles.push({
      name,
      label: humanizeJoint(name),
      type: joint.jointType,
      lower: continuous ? -CONTINUOUS_RANGE : joint.limit.lower,
      upper: continuous ? CONTINUOUS_RANGE : joint.limit.upper,
      value: joint.angle,
    });
  }
  return handles;
}

export function clampToJoint(handle: JointHandle, value: number): number {
  return Math.min(handle.upper, Math.max(handle.lower, value));
}

export function jointStep(handle: JointHandle): number {
  return (handle.upper - handle.lower) / 200 || 0.01;
}

export function formatJointValue(handle: JointHandle, value: number): string {
  return handle.type === "prismatic"
    ? `${(value * 1000).toFixed(0)} mm`
    : `${value.toFixed(2)} rad`;
}
