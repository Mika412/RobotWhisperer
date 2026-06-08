# Robot model attributions

The 3D robot models bundled under `static/assets/<robot>/` are third-party
assets, redistributed here under their original permissive licenses. Each
robot directory keeps the upstream `LICENSE` file verbatim. Only the visual
meshes are vendored; collision geometry is stripped and mesh URIs are rewritten
to a flat `package://<dir>/meshes/visual/...` layout (see
`scripts/prep-robot-assets.py` for the exact, reproducible derivation).

## UR10e (`ur10e/`)

- **Source:** Universal Robots — `Universal_Robots_ROS2_Description`
  (`ur_description`).
- **Upstream:** https://github.com/UniversalRobots/Universal_Robots_ROS2_Description
- **Commit:** `22f055da2fa7e2158254426107d1f257fd56aebb`
- **License:** BSD-3-Clause (`ur10e/LICENSE`).
- **Copyright:** © Universal Robots A/S.

## Franka Panda (`franka_panda/`)

- **Source:** vendored via Gepetto `example-robot-data` (`panda_description`),
  itself derived from Franka Emika's `franka_description`.
- **Upstream:** https://github.com/Gepetto/example-robot-data
  (original: https://github.com/frankaemika/franka_ros)
- **Commit:** `d0d9098d752014aec3725b07766962acf06c5418`
- **License:** Apache-2.0 (`franka_panda/LICENSE`).
- **Copyright:** © Franka Emika GmbH.

## KUKA LBR iiwa 14 (`iiwa14/`)

- **Source:** RobotLocomotion `drake` (`manipulation/models/iiwa_description`).
- **Upstream:** https://github.com/RobotLocomotion/drake
- **Commit:** `7abea0556ede980a5077fe1a8cfbae59b57c7c27`
- **License:** BSD-3-Clause (`iiwa14/LICENSE`).
- **Copyright:** © Toyota Research Institute / KUKA.

## Allegro Hand (`allegro_hand/`)

- **Source:** RobotLocomotion `drake` (`manipulation/models/allegro_hand_description`);
  meshes derive from Wonik (SimLab) `allegro_hand_ros`.
- **Upstream:** https://github.com/RobotLocomotion/drake
  (original: https://github.com/simlabrobotics/allegro_hand_ros)
- **Commit:** `7abea0556ede980a5077fe1a8cfbae59b57c7c27`
- **License:** BSD-3-Clause (`allegro_hand/LICENSE`).
- **Copyright:** © Toyota Research Institute / Wonik Robotics.

---

To regenerate or add robots, run `python3 scripts/prep-robot-assets.py` from the
repo root (requires `robot_descriptions` and, for UR, a ROS `xacro` on `PATH`).
