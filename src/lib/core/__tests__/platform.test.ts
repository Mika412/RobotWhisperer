import { describe, it, expect, afterEach } from "vitest";
import { isMobileDevice } from "../platform";

interface FakeNavigator {
  userAgent: string;
  maxTouchPoints: number;
  userAgentData?: { mobile?: boolean };
}

function setNavigator(nav: FakeNavigator): void {
  Object.defineProperty(globalThis, "navigator", { value: nav, configurable: true });
}

const IPHONE =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1";
const ANDROID_PHONE =
  "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36";
const ANDROID_TABLET =
  "Mozilla/5.0 (Linux; Android 13; SM-X710) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
const IPAD_LEGACY =
  "Mozilla/5.0 (iPad; CPU OS 12_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/12.1 Mobile/15E148 Safari/604.1";
const IPADOS_DESKTOP_UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";
const MAC_DESKTOP =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
const WINDOWS_DESKTOP =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

afterEach(() => {
  Object.defineProperty(globalThis, "navigator", {
    value: { userAgent: WINDOWS_DESKTOP, maxTouchPoints: 0 },
    configurable: true,
  });
});

describe("isMobileDevice", () => {
  it("detects iPhone", () => {
    setNavigator({ userAgent: IPHONE, maxTouchPoints: 5 });
    expect(isMobileDevice()).toBe(true);
  });

  it("detects Android phones", () => {
    setNavigator({ userAgent: ANDROID_PHONE, maxTouchPoints: 5 });
    expect(isMobileDevice()).toBe(true);
  });

  it("allows Android tablets, which omit the Mobile token", () => {
    setNavigator({ userAgent: ANDROID_TABLET, maxTouchPoints: 5 });
    expect(isMobileDevice()).toBe(false);
  });

  it("allows iPads, including iPadOS 13+ that reports a desktop Safari UA", () => {
    setNavigator({ userAgent: IPAD_LEGACY, maxTouchPoints: 5 });
    expect(isMobileDevice()).toBe(false);
    setNavigator({ userAgent: IPADOS_DESKTOP_UA, maxTouchPoints: 5 });
    expect(isMobileDevice()).toBe(false);
  });

  it("honours the UA Client Hints mobile flag", () => {
    setNavigator({
      userAgent: WINDOWS_DESKTOP,
      maxTouchPoints: 0,
      userAgentData: { mobile: true },
    });
    expect(isMobileDevice()).toBe(true);
  });

  it("does not flag a real Mac or Windows desktop", () => {
    setNavigator({ userAgent: MAC_DESKTOP, maxTouchPoints: 0 });
    expect(isMobileDevice()).toBe(false);
    setNavigator({ userAgent: WINDOWS_DESKTOP, maxTouchPoints: 0 });
    expect(isMobileDevice()).toBe(false);
  });

  it("does not flag a touchscreen Windows laptop", () => {
    setNavigator({ userAgent: WINDOWS_DESKTOP, maxTouchPoints: 10 });
    expect(isMobileDevice()).toBe(false);
  });
});
