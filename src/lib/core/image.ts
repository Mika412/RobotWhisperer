export function rgb8ToRgba(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
): Uint8ClampedArray {
  const dst = new Uint8ClampedArray(width * height * 4);
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const si = srcRowOffset + col * 3;
      const di = dstRowOffset + col * 4;
      dst[di] = src[si];
      dst[di + 1] = src[si + 1];
      dst[di + 2] = src[si + 2];
      dst[di + 3] = 255;
    }
  }
  return dst;
}

export function bgr8ToRgba(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
): Uint8ClampedArray {
  const dst = new Uint8ClampedArray(width * height * 4);
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const si = srcRowOffset + col * 3;
      const di = dstRowOffset + col * 4;
      dst[di] = src[si + 2];
      dst[di + 1] = src[si + 1];
      dst[di + 2] = src[si];
      dst[di + 3] = 255;
    }
  }
  return dst;
}

export function mono8ToRgba(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
): Uint8ClampedArray {
  const dst = new Uint8ClampedArray(width * height * 4);
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const si = srcRowOffset + col;
      const di = dstRowOffset + col * 4;
      const v = src[si];
      dst[di] = v;
      dst[di + 1] = v;
      dst[di + 2] = v;
      dst[di + 3] = 255;
    }
  }
  return dst;
}

export function rgba8ToRgba(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
): Uint8ClampedArray {
  if (step === width * 4) {
    return new Uint8ClampedArray(src.buffer, src.byteOffset, width * height * 4);
  }
  const dst = new Uint8ClampedArray(width * height * 4);
  const rowBytes = width * 4;
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * rowBytes;
    dst.set(src.subarray(srcRowOffset, srcRowOffset + rowBytes), dstRowOffset);
  }
  return dst;
}

export function bgra8ToRgba(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
): Uint8ClampedArray {
  const dst = new Uint8ClampedArray(width * height * 4);
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const si = srcRowOffset + col * 4;
      const di = dstRowOffset + col * 4;
      dst[di] = src[si + 2];
      dst[di + 1] = src[si + 1];
      dst[di + 2] = src[si];
      dst[di + 3] = src[si + 3];
    }
  }
  return dst;
}

export const SUPPORTED_ENCODINGS = ["rgb8", "bgr8", "mono8", "rgba8", "bgra8"] as const;
export type SupportedEncoding = (typeof SUPPORTED_ENCODINGS)[number];

export function convertToRgba(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
  encoding: string,
): Uint8ClampedArray | null {
  switch (encoding) {
    case "rgb8":
      return rgb8ToRgba(src, width, height, step);
    case "bgr8":
      return bgr8ToRgba(src, width, height, step);
    case "mono8":
      return mono8ToRgba(src, width, height, step);
    case "rgba8":
      return rgba8ToRgba(src, width, height, step);
    case "bgra8":
      return bgra8ToRgba(src, width, height, step);
    default:
      return null;
  }
}

export function convertToRgbaInto(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
  encoding: string,
  dst: Uint8ClampedArray,
): boolean {
  switch (encoding) {
    case "rgb8":
      rgb8ToRgbaInto(src, width, height, step, dst);
      return true;
    case "bgr8":
      bgr8ToRgbaInto(src, width, height, step, dst);
      return true;
    case "mono8":
      mono8ToRgbaInto(src, width, height, step, dst);
      return true;
    case "rgba8":
      rgba8ToRgbaInto(src, width, height, step, dst);
      return true;
    case "bgra8":
      bgra8ToRgbaInto(src, width, height, step, dst);
      return true;
    default:
      return false;
  }
}

function rgb8ToRgbaInto(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
  dst: Uint8ClampedArray,
): void {
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const si = srcRowOffset + col * 3;
      const di = dstRowOffset + col * 4;
      dst[di] = src[si];
      dst[di + 1] = src[si + 1];
      dst[di + 2] = src[si + 2];
      dst[di + 3] = 255;
    }
  }
}

function bgr8ToRgbaInto(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
  dst: Uint8ClampedArray,
): void {
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const si = srcRowOffset + col * 3;
      const di = dstRowOffset + col * 4;
      dst[di] = src[si + 2];
      dst[di + 1] = src[si + 1];
      dst[di + 2] = src[si];
      dst[di + 3] = 255;
    }
  }
}

function mono8ToRgbaInto(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
  dst: Uint8ClampedArray,
): void {
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const v = src[srcRowOffset + col];
      const di = dstRowOffset + col * 4;
      dst[di] = v;
      dst[di + 1] = v;
      dst[di + 2] = v;
      dst[di + 3] = 255;
    }
  }
}

function rgba8ToRgbaInto(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
  dst: Uint8ClampedArray,
): void {
  if (step === width * 4) {
    dst.set(src.subarray(0, width * height * 4));
  } else {
    const rowBytes = width * 4;
    for (let row = 0; row < height; row++) {
      dst.set(src.subarray(row * step, row * step + rowBytes), row * rowBytes);
    }
  }
}

function bgra8ToRgbaInto(
  src: Uint8Array,
  width: number,
  height: number,
  step: number,
  dst: Uint8ClampedArray,
): void {
  for (let row = 0; row < height; row++) {
    const srcRowOffset = row * step;
    const dstRowOffset = row * width * 4;
    for (let col = 0; col < width; col++) {
      const si = srcRowOffset + col * 4;
      const di = dstRowOffset + col * 4;
      dst[di] = src[si + 2];
      dst[di + 1] = src[si + 1];
      dst[di + 2] = src[si];
      dst[di + 3] = src[si + 3];
    }
  }
}
