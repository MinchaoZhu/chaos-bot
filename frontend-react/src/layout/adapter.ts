import { useEffect, useState } from "react";

export type LayoutMode = "desktop" | "mobile";

export interface LayoutAdapter {
  mode: LayoutMode;
  isDesktop: boolean;
  isMobile: boolean;
}

const MOBILE_BREAKPOINT = 980;

function detectMode(width: number): LayoutMode {
  return width >= MOBILE_BREAKPOINT ? "desktop" : "mobile";
}

export function useLayoutAdapter(): LayoutAdapter {
  const [mode, setMode] = useState<LayoutMode>(() => detectMode(window.innerWidth));

  useEffect(() => {
    const onResize = () => {
      setMode(detectMode(window.innerWidth));
    };

    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return {
    mode,
    isDesktop: mode === "desktop",
    isMobile: mode === "mobile",
  };
}
