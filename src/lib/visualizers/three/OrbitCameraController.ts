import * as THREE from "three";

interface OrbitOptions {
  distance?: number;
  minDistance?: number;
  maxDistance?: number;
  groundLevel?: number;
  onChange?: () => void;
}

const DEFAULT_AZIMUTH = Math.PI * 0.25;
const DEFAULT_POLAR = Math.PI * 0.36;
const POLAR_MIN = 0.02;
const POLAR_MAX = Math.PI - 0.02;
const GROUND_MARGIN = 0.02;

export class OrbitCameraController {
  readonly target = new THREE.Vector3(0, 0, 0);

  private azimuth = DEFAULT_AZIMUTH;
  private polar = DEFAULT_POLAR;
  private distance: number;
  private minDistance: number;
  private maxDistance: number;

  private activePointer: number | null = null;
  private panning = false;
  private lastX = 0;
  private lastY = 0;
  private readonly pointers = new Map<number, { x: number; y: number }>();
  private twoFinger = false;
  private lastPinch = 0;
  private lastMidX = 0;
  private lastMidY = 0;
  private readonly groundLevel?: number;
  private readonly onChange?: () => void;

  constructor(
    private readonly camera: THREE.PerspectiveCamera,
    private readonly dom: HTMLElement,
    options: OrbitOptions = {},
  ) {
    this.distance = options.distance ?? 6;
    this.minDistance = options.minDistance ?? 0.4;
    this.maxDistance = options.maxDistance ?? 250;
    this.groundLevel = options.groundLevel;
    this.onChange = options.onChange;
    this.dom.style.touchAction = "none";
    this.dom.addEventListener("pointerdown", this.onPointerDown);
    this.dom.addEventListener("wheel", this.onWheel, { passive: false });
    this.dom.addEventListener("contextmenu", this.onContextMenu);
    this.place();
  }

  dispose(): void {
    this.dom.removeEventListener("pointerdown", this.onPointerDown);
    this.dom.removeEventListener("wheel", this.onWheel);
    this.dom.removeEventListener("contextmenu", this.onContextMenu);
    window.removeEventListener("pointermove", this.onPointerMove);
    window.removeEventListener("pointerup", this.onPointerUp);
    window.removeEventListener("pointercancel", this.onPointerUp);
  }

  private clampDistance(value: number): number {
    return Math.min(this.maxDistance, Math.max(this.minDistance, value));
  }

  setDistanceLimits(min: number, max: number): void {
    this.minDistance = min;
    this.maxDistance = max;
    this.distance = this.clampDistance(this.distance);
  }

  frame(center: THREE.Vector3, distance: number): void {
    this.target.copy(center);
    this.distance = this.clampDistance(distance);
    this.azimuth = DEFAULT_AZIMUTH;
    this.polar = DEFAULT_POLAR;
    this.place();
  }

  private place(): void {
    const sinPolar = Math.sin(this.polar);
    this.camera.position.set(
      this.target.x + this.distance * sinPolar * Math.sin(this.azimuth),
      this.target.y + this.distance * Math.cos(this.polar),
      this.target.z + this.distance * sinPolar * Math.cos(this.azimuth),
    );
    if (this.groundLevel !== undefined) {
      this.camera.position.y = Math.max(this.camera.position.y, this.groundLevel + GROUND_MARGIN);
    }
    this.camera.lookAt(this.target);
    this.onChange?.();
  }

  private onContextMenu = (event: Event): void => {
    event.preventDefault();
  };

  private onPointerDown = (event: PointerEvent): void => {
    this.pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    if (this.pointers.size === 1) {
      this.activePointer = event.pointerId;
      this.panning = event.button === 1 || event.button === 2 || event.shiftKey;
      this.lastX = event.clientX;
      this.lastY = event.clientY;
      window.addEventListener("pointermove", this.onPointerMove);
      window.addEventListener("pointerup", this.onPointerUp);
      window.addEventListener("pointercancel", this.onPointerUp);
    } else if (this.pointers.size === 2) {
      this.twoFinger = false;
    }
  };

  private onPointerMove = (event: PointerEvent): void => {
    if (!this.pointers.has(event.pointerId)) return;
    this.pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });

    if (this.pointers.size >= 2) {
      const [a, b] = [...this.pointers.values()];
      const dist = Math.hypot(a.x - b.x, a.y - b.y);
      const midX = (a.x + b.x) / 2;
      const midY = (a.y + b.y) / 2;
      if (!this.twoFinger) {
        this.twoFinger = true;
      } else {
        if (this.lastPinch > 0 && dist > 0) {
          this.distance = this.clampDistance(this.distance * (this.lastPinch / dist));
        }
        this.pan(midX - this.lastMidX, midY - this.lastMidY);
        this.place();
      }
      this.lastPinch = dist;
      this.lastMidX = midX;
      this.lastMidY = midY;
      return;
    }

    if (event.pointerId !== this.activePointer) return;
    const dx = event.clientX - this.lastX;
    const dy = event.clientY - this.lastY;
    this.lastX = event.clientX;
    this.lastY = event.clientY;
    if (this.panning) {
      this.pan(dx, dy);
    } else {
      this.azimuth -= dx * 0.005;
      this.polar = Math.min(POLAR_MAX, Math.max(POLAR_MIN, this.polar - dy * 0.005));
    }
    this.place();
  };

  private onPointerUp = (event: PointerEvent): void => {
    if (!this.pointers.delete(event.pointerId)) return;
    if (this.pointers.size < 2) this.twoFinger = false;
    if (event.pointerId === this.activePointer && this.pointers.size >= 1) {
      const [next, point] = [...this.pointers.entries()][0];
      this.activePointer = next;
      this.lastX = point.x;
      this.lastY = point.y;
    }
    if (this.pointers.size === 0) {
      this.activePointer = null;
      window.removeEventListener("pointermove", this.onPointerMove);
      window.removeEventListener("pointerup", this.onPointerUp);
      window.removeEventListener("pointercancel", this.onPointerUp);
    }
  };

  private onWheel = (event: WheelEvent): void => {
    event.preventDefault();
    const factor = Math.exp(event.deltaY * 0.001);
    this.distance = this.clampDistance(this.distance * factor);
    this.place();
  };

  private pan(dx: number, dy: number): void {
    const right = new THREE.Vector3().setFromMatrixColumn(this.camera.matrix, 0);
    const up = new THREE.Vector3().setFromMatrixColumn(this.camera.matrix, 1);
    const scale = this.distance * 0.0018;
    this.target.addScaledVector(right, -dx * scale);
    this.target.addScaledVector(up, dy * scale);
  }
}
