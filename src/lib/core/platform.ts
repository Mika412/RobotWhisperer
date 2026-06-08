export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const isBrowser = (): boolean =>
  typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

type UaDataNavigator = Navigator & { userAgentData?: { mobile?: boolean } };

export const isMobileDevice = (): boolean => {
  if (typeof navigator === "undefined") return false;
  if (isTauri()) return false;

  const ua = navigator.userAgent;

  if (/iPhone|iPod/i.test(ua)) return true;
  if (/Android/i.test(ua)) return /\bMobile\b/i.test(ua);

  const uaData = (navigator as UaDataNavigator).userAgentData;
  if (uaData?.mobile === true) return true;

  return /webOS|BlackBerry|IEMobile|Opera Mini|Windows Phone/i.test(ua);
};
