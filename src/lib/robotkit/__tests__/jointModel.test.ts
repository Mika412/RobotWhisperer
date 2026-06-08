import { describe, it, expect } from "vitest";
import type { URDFRobot } from "urdf-loader";
import { clampToJoint, describeJoints, formatJointValue, jointStep } from "../jointModel";

function fakeRobot(): URDFRobot {
  return {
    joints: {
      base_link: { jointType: "fixed", limit: { lower: 0, upper: 0 }, angle: 0 },
      shoulder_pan_joint: { jointType: "revolute", limit: { lower: -2, upper: 2 }, angle: 0.5 },
      wrist_continuous: { jointType: "continuous", limit: { lower: 0, upper: 0 }, angle: 0 },
      panda_finger_joint1: {
        jointType: "prismatic",
        limit: { lower: 0, upper: 0.04 },
        angle: 0.01,
      },
    },
  } as unknown as URDFRobot;
}

describe("describeJoints", () => {
  it("keeps only controllable joints in URDF order", () => {
    const handles = describeJoints(fakeRobot());
    expect(handles.map((handle) => handle.name)).toEqual([
      "shoulder_pan_joint",
      "wrist_continuous",
      "panda_finger_joint1",
    ]);
  });

  it("clamps continuous joints to +/- pi and keeps real limits otherwise", () => {
    const handles = describeJoints(fakeRobot());
    const continuous = handles.find((handle) => handle.name === "wrist_continuous");
    expect(continuous).toMatchObject({ lower: -Math.PI, upper: Math.PI });
    const revolute = handles.find((handle) => handle.name === "shoulder_pan_joint");
    expect(revolute).toMatchObject({ lower: -2, upper: 2, value: 0.5 });
  });

  it("humanizes joint names into readable labels", () => {
    const handles = describeJoints(fakeRobot());
    const labels = Object.fromEntries(handles.map((handle) => [handle.name, handle.label]));
    expect(labels.shoulder_pan_joint).toBe("Shoulder Pan Joint");
    expect(labels.panda_finger_joint1).toBe("Panda Finger Joint 1");
  });
});

describe("clampToJoint", () => {
  it("clamps values into the joint limits", () => {
    const [handle] = describeJoints(fakeRobot());
    expect(clampToJoint(handle, 5)).toBe(2);
    expect(clampToJoint(handle, -5)).toBe(-2);
    expect(clampToJoint(handle, 1)).toBe(1);
  });
});

describe("jointStep", () => {
  it("derives a fine step across the range", () => {
    const [handle] = describeJoints(fakeRobot());
    expect(jointStep(handle)).toBeCloseTo(4 / 200);
  });
});

describe("formatJointValue", () => {
  it("renders prismatic joints in millimetres and rotary joints in radians", () => {
    const handles = describeJoints(fakeRobot());
    const prismatic = handles.find((handle) => handle.type === "prismatic")!;
    const revolute = handles.find((handle) => handle.type === "revolute")!;
    expect(formatJointValue(prismatic, 0.025)).toBe("25 mm");
    expect(formatJointValue(revolute, 1.234)).toBe("1.23 rad");
  });
});
