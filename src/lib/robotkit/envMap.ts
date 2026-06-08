import * as THREE from "three";

let studio: THREE.Scene | null = null;

const GRADIENT_VERTEX = /* glsl */ `
  varying vec3 vDirection;
  void main() {
    vDirection = normalize(position);
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const GRADIENT_FRAGMENT = /* glsl */ `
  precision highp float;
  varying vec3 vDirection;
  uniform vec3 uFloor;
  uniform vec3 uHorizon;
  uniform vec3 uSky;

  void main() {
    float height = normalize(vDirection).y;
    vec3 lower = mix(uHorizon, uFloor, smoothstep(0.0, -0.55, height));
    vec3 color = mix(lower, uSky, smoothstep(0.0, 0.6, height));
    gl_FragColor = vec4(color, 1.0);
  }
`;

function gradientBackdrop(): THREE.Mesh {
  const material = new THREE.ShaderMaterial({
    side: THREE.BackSide,
    depthWrite: false,
    uniforms: {
      uFloor: { value: new THREE.Color(0x05070b) },
      uHorizon: { value: new THREE.Color(0x1a1f29) },
      uSky: { value: new THREE.Color(0x3a4350) },
    },
    vertexShader: GRADIENT_VERTEX,
    fragmentShader: GRADIENT_FRAGMENT,
  });
  return new THREE.Mesh(new THREE.SphereGeometry(40, 32, 16), material);
}

function softbox(
  scene: THREE.Scene,
  intensity: number,
  width: number,
  height: number,
  position: THREE.Vector3,
): void {
  const panel = new THREE.Mesh(
    new THREE.PlaneGeometry(width, height),
    new THREE.MeshBasicMaterial({ color: new THREE.Color(intensity, intensity, intensity) }),
  );
  panel.position.copy(position);
  panel.lookAt(0, 0, 0);
  scene.add(panel);
}

function studioScene(): THREE.Scene {
  if (studio) return studio;
  const scene = new THREE.Scene();
  scene.add(gradientBackdrop());

  softbox(scene, 9.0, 7, 4, new THREE.Vector3(3.5, 7, 4.5));
  softbox(scene, 5.5, 1.2, 9, new THREE.Vector3(5.5, 4, -1));
  softbox(scene, 4.0, 9, 1.4, new THREE.Vector3(-2, 6.5, -3));
  softbox(scene, 2.4, 8, 8, new THREE.Vector3(-6.5, 2, 4));
  softbox(scene, 3.2, 1.0, 8, new THREE.Vector3(-4.5, 3, -4));
  softbox(scene, 1.4, 12, 5, new THREE.Vector3(0, -5, 5));

  studio = scene;
  return scene;
}

export function environmentTexture(renderer: THREE.WebGLRenderer): THREE.Texture {
  const generator = new THREE.PMREMGenerator(renderer);
  const texture = generator.fromScene(studioScene(), 0.01).texture;
  generator.dispose();
  return texture;
}
