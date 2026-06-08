"""Regenerate the vendored robot assets under static/assets/.

This documents the provenance of every committed mesh/URDF. It fetches the
upstream description packages via robot_descriptions, generates a plain URDF
(running xacro for UR), rewrites mesh URIs to a flat package://<dir>/meshes/...
layout, strips collision geometry (the viewer only renders visual), and copies
the visual meshes plus the upstream LICENSE into static/assets/<dir>/.

Run from the repo root:  python3 scripts/prep-robot-assets.py
Requires: python3, robot_descriptions, and (for UR) a ROS xacro on PATH.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ASSETS = REPO_ROOT / "static" / "assets"
CACHE = Path.home() / ".cache" / "robot_descriptions"

COLLISION_BLOCK = re.compile(r"[ \t]*<collision\b.*?</collision>\s*", re.DOTALL)
GAZEBO_BLOCK = re.compile(r"[ \t]*<gazebo\b.*?</gazebo>\s*", re.DOTALL)


def strip_collision(urdf: str) -> str:
    return COLLISION_BLOCK.sub("", urdf)


def strip_gazebo(urdf: str) -> str:
    return GAZEBO_BLOCK.sub("", urdf)


INLINE_MATERIAL = re.compile(
    r'<material\s+name="([^"]+)"\s*>\s*<color\s+rgba="([^"]+)"\s*/>\s*</material>'
)
ROBOT_OPEN = re.compile(r"(<robot\b[^>]*>)")


def hoist_materials(urdf: str) -> str:
    """Promote inline named-material colour definitions to top-level <material>
    elements. urdf-loader only resolves `<material name="x"/>` references against
    top-level definitions, but some URDFs define the colour inline on first use
    and reference by name afterwards (so every later reference renders default
    white)."""
    colors: dict[str, str] = {}
    for name, rgba in INLINE_MATERIAL.findall(urdf):
        colors.setdefault(name, rgba)
    if not colors:
        return urdf
    block = "".join(
        f'\n  <material name="{name}"><color rgba="{rgba}"/></material>'
        for name, rgba in colors.items()
    )
    return ROBOT_OPEN.sub(lambda match: match.group(1) + block, urdf, count=1)


def write_urdf(directory: str, urdf: str) -> None:
    target = ASSETS / directory
    (target / "meshes" / "visual").mkdir(parents=True, exist_ok=True)
    (target / f"{directory}.urdf").write_text(urdf)


def copy_meshes(src_dir: Path, directory: str, rel: str = "meshes/visual") -> None:
    dest = ASSETS / directory / rel
    dest.mkdir(parents=True, exist_ok=True)
    for mesh in sorted(src_dir.iterdir()):
        if mesh.suffix.lower() in (".dae", ".stl", ".obj"):
            shutil.copy2(mesh, dest / mesh.name)


def copy_license(src: Path, directory: str, name: str = "LICENSE") -> None:
    shutil.copy2(src, ASSETS / directory / name)


def prep_ur10e() -> None:
    from robot_descriptions import ur10e_description as ur

    pkg = Path(ur.PACKAGE_PATH)
    overlay = Path("/tmp/ur_overlay")
    (overlay / "share" / "ament_index" / "resource_index" / "packages").mkdir(
        parents=True, exist_ok=True
    )
    (overlay / "share" / "ament_index" / "resource_index" / "packages" / "ur_description").touch()
    link = overlay / "share" / "ur_description"
    if link.is_symlink() or link.exists():
        link.unlink()
    link.symlink_to(pkg)

    env = dict(os.environ)
    env["AMENT_PREFIX_PATH"] = f"{overlay}:{env.get('AMENT_PREFIX_PATH', '')}"
    urdf = subprocess.check_output(
        ["xacro", str(ur.XACRO_PATH), "ur_type:=ur10e", "name:=ur10e"], env=env, text=True
    )
    urdf = strip_collision(urdf)
    urdf = urdf.replace(
        "package://ur_description/meshes/ur10e/visual/", "package://ur10e/meshes/visual/"
    )
    write_urdf("ur10e", urdf)
    copy_meshes(pkg / "meshes" / "ur10e" / "visual", "ur10e")
    copy_license(pkg / "LICENSE", "ur10e")


def prep_panda() -> None:
    from robot_descriptions import panda_description as pa

    pkg = Path(pa.PACKAGE_PATH)
    urdf = Path(pa.URDF_PATH).read_text()
    urdf = strip_collision(urdf)
    urdf = urdf.replace(
        "package://example-robot-data/robots/panda_description/meshes/visual/",
        "package://franka_panda/meshes/visual/",
    )
    write_urdf("franka_panda", urdf)
    copy_meshes(pkg / "meshes" / "visual", "franka_panda")
    copy_license(pkg / "LICENSE", "franka_panda")


def prep_iiwa14() -> None:
    from robot_descriptions import iiwa14_description as iiwa

    pkg = Path(iiwa.PACKAGE_PATH)
    urdf = Path(iiwa.URDF_PATH_NO_COLLISION).read_text()
    urdf = strip_gazebo(strip_collision(urdf))
    urdf = urdf.replace(
        "package://drake/manipulation/models/iiwa_description/meshes/visual/",
        "package://iiwa14/meshes/visual/",
    )
    write_urdf("iiwa14", urdf)
    copy_meshes(pkg / "meshes" / "visual", "iiwa14")
    copy_license(Path(iiwa.REPOSITORY_PATH) / "LICENSE.TXT", "iiwa14")


def prep_allegro() -> None:
    from robot_descriptions import allegro_hand_description as allegro

    pkg = Path(allegro.PACKAGE_PATH)
    urdf = Path(allegro.URDF_PATH_RIGHT).read_text()
    urdf = hoist_materials(strip_gazebo(strip_collision(urdf)))
    # Drake declares the hand's "black" as 0.2 grey; the real Allegro Hand is a
    # matte black, so correct the albedo to read true to the physical hardware.
    urdf = urdf.replace('rgba="0.2 0.2 0.2 1"', 'rgba="0.045 0.045 0.045 1"')
    urdf = urdf.replace(
        "package://drake/manipulation/models/allegro_hand_description/meshes/",
        "package://allegro_hand/meshes/",
    )
    write_urdf("allegro_hand", urdf)
    copy_meshes(pkg / "meshes", "allegro_hand", rel="meshes")
    copy_license(Path(allegro.REPOSITORY_PATH) / "LICENSE.TXT", "allegro_hand")



def main() -> None:
    prep_ur10e()
    prep_panda()
    prep_iiwa14()
    prep_allegro()
    print("Vendored assets written under", ASSETS)


if __name__ == "__main__":
    main()
