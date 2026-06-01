import { describe, it, expect } from "vitest";
import {
  rgb8ToRgba,
  bgr8ToRgba,
  mono8ToRgba,
  rgba8ToRgba,
  bgra8ToRgba,
  convertToRgba,
} from "../image";

describe("image pixel converters", () => {
  describe("rgb8ToRgba", () => {
    it("converts 2x2 rgb8 image to RGBA", () => {
      const src = new Uint8Array([255, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128]);
      const result = rgb8ToRgba(src, 2, 2, 6);
      expect(result).toEqual(
        new Uint8ClampedArray([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 128, 128, 128, 255]),
      );
    });

    it("handles row padding (step > width * 3)", () => {
      const src = new Uint8Array([10, 20, 30, 40, 50, 60, 0xaa, 0xbb]);
      const result = rgb8ToRgba(src, 2, 1, 8);
      expect(result).toEqual(new Uint8ClampedArray([10, 20, 30, 255, 40, 50, 60, 255]));
    });
  });

  describe("bgr8ToRgba", () => {
    it("swaps B and R channels", () => {
      const src = new Uint8Array([100, 150, 200]);
      const result = bgr8ToRgba(src, 1, 1, 3);
      expect(result).toEqual(new Uint8ClampedArray([200, 150, 100, 255]));
    });

    it("converts 2x1 bgr8 image", () => {
      const src = new Uint8Array([0, 0, 255, 255, 0, 0]);
      const result = bgr8ToRgba(src, 2, 1, 6);
      expect(result).toEqual(new Uint8ClampedArray([255, 0, 0, 255, 0, 0, 255, 255]));
    });
  });

  describe("mono8ToRgba", () => {
    it("expands grayscale to RGB with A=255", () => {
      const src = new Uint8Array([0, 128, 255]);
      const result = mono8ToRgba(src, 3, 1, 3);
      expect(result).toEqual(
        new Uint8ClampedArray([0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255]),
      );
    });

    it("handles row padding", () => {
      const src = new Uint8Array([50, 100, 0xaa, 0xbb, 200, 250, 0xcc, 0xdd]);
      const result = mono8ToRgba(src, 2, 2, 4);
      expect(result).toEqual(
        new Uint8ClampedArray([
          50, 50, 50, 255, 100, 100, 100, 255, 200, 200, 200, 255, 250, 250, 250, 255,
        ]),
      );
    });
  });

  describe("rgba8ToRgba", () => {
    it("passes through when step equals width*4 (zero-copy path)", () => {
      const src = new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80]);
      const result = rgba8ToRgba(src, 2, 1, 8);
      expect(Array.from(result)).toEqual([10, 20, 30, 40, 50, 60, 70, 80]);
    });

    it("handles row padding", () => {
      const src = new Uint8Array([
        10, 20, 30, 128, 0xaa, 0xbb, 0xcc, 0xdd, 40, 50, 60, 200, 0xee, 0xff, 0x11, 0x22,
      ]);
      const result = rgba8ToRgba(src, 1, 2, 8);
      expect(result).toEqual(new Uint8ClampedArray([10, 20, 30, 128, 40, 50, 60, 200]));
    });
  });

  describe("bgra8ToRgba", () => {
    it("swaps B and R channels, preserves A", () => {
      const src = new Uint8Array([100, 150, 200, 128]);
      const result = bgra8ToRgba(src, 1, 1, 4);
      expect(result).toEqual(new Uint8ClampedArray([200, 150, 100, 128]));
    });

    it("handles multiple pixels", () => {
      const src = new Uint8Array([255, 0, 0, 255, 0, 255, 0, 128]);
      const result = bgra8ToRgba(src, 2, 1, 8);
      expect(result).toEqual(new Uint8ClampedArray([0, 0, 255, 255, 0, 255, 0, 128]));
    });
  });

  describe("convertToRgba", () => {
    it("dispatches to correct converter", () => {
      const src = new Uint8Array([255, 0, 0]);
      const result = convertToRgba(src, 1, 1, 3, "rgb8");
      expect(result).toEqual(new Uint8ClampedArray([255, 0, 0, 255]));
    });

    it("returns null for unsupported encoding", () => {
      const src = new Uint8Array([0]);
      expect(convertToRgba(src, 1, 1, 1, "bayer_rggb8")).toBeNull();
    });

    it("handles all supported encodings without throwing", () => {
      const src = new Uint8Array(16);
      expect(convertToRgba(src, 1, 1, 3, "rgb8")).not.toBeNull();
      expect(convertToRgba(src, 1, 1, 3, "bgr8")).not.toBeNull();
      expect(convertToRgba(src, 1, 1, 1, "mono8")).not.toBeNull();
      expect(convertToRgba(src, 1, 1, 4, "rgba8")).not.toBeNull();
      expect(convertToRgba(src, 1, 1, 4, "bgra8")).not.toBeNull();
    });
  });
});
