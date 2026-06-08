import { describe, it, expect, vi } from "vitest";
import type { URDFRobot } from "urdf-loader";
import { createJointDriver } from "../jointDriver";

function fakeRobot() {
  const applied: Record<string, number> = {};
  const joint = (name: string) => ({
    angle: applied[name] ?? 0,
    setJointValue: (value: number) => {
      applied[name] = value;
    },
  });
  const robot = {
    joints: {
      get joint1() {
        return joint("joint1");
      },
      get joint2() {
        return joint("joint2");
      },
    },
  } as unknown as URDFRobot;
  return { robot, applied };
}

describe("createJointDriver", () => {
  it("sets a single joint and notifies the renderer", () => {
    const { robot, applied } = fakeRobot();
    const onChange = vi.fn();
    const driver = createJointDriver(robot, onChange);
    driver.setJoint("joint1", 0.7);
    expect(applied.joint1).toBe(0.7);
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("ignores unknown joints without notifying", () => {
    const { robot } = fakeRobot();
    const onChange = vi.fn();
    const driver = createJointDriver(robot, onChange);
    driver.setJoint("missing", 1);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("maps a JointState name/position pair onto the matching joints", () => {
    const { robot, applied } = fakeRobot();
    const onChange = vi.fn();
    const driver = createJointDriver(robot, onChange);
    driver.applyNamedPositions(["joint2", "joint1", "absent"], [0.2, -0.3, 9]);
    expect(applied).toEqual({ joint2: 0.2, joint1: -0.3 });
    expect(onChange).toHaveBeenCalledTimes(1);
  });
});
