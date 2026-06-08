import * as THREE from "three";

const VERTEX = /* glsl */ `
  varying vec3 vWorld;
  void main() {
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

const FRAGMENT = /* glsl */ `
  precision highp float;
  varying vec3 vWorld;
  uniform vec3 uBase;
  uniform vec3 uLine;
  uniform vec3 uBackground;

  float gridLine(vec2 point, float scale, float widthPx) {
    vec2 grid = abs(fract(point * scale - 0.5) - 0.5) / fwidth(point * scale);
    return 1.0 - smoothstep(0.0, widthPx, min(grid.x, grid.y));
  }

  void main() {
    vec2 point = vWorld.xz;
    float major = gridLine(point, 5.0, 1.3);
    float minor = gridLine(point, 20.0, 1.0) * 0.25;
    float lines = clamp(major + minor, 0.0, 1.0);

    vec3 color = mix(uBase, uLine, lines);
    float radius = length(point);
    float vignette = 1.0 - smoothstep(0.9, 2.4, radius);
    color = mix(uBackground, color, vignette);
    gl_FragColor = vec4(color, vignette);
  }
`;

export function createFloor(): THREE.Mesh {
  const material = new THREE.ShaderMaterial({
    uniforms: {
      uBase: { value: new THREE.Color(0x252b34) },
      uLine: { value: new THREE.Color(0x5a6577) },
      uBackground: { value: new THREE.Color(0x1b202a) },
    },
    vertexShader: VERTEX,
    fragmentShader: FRAGMENT,
    transparent: true,
  });
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(8, 8), material);
  floor.rotation.x = -Math.PI / 2;
  floor.receiveShadow = true;
  return floor;
}
