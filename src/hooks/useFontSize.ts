import { useCallback, useEffect, useState } from "react";

export type FontSize = "compact" | "normal" | "large";

const STORAGE_KEY = "ptv:font-size";
const DEFAULT_FONT_SIZE: FontSize = "normal";

function getStoredFontSize(): FontSize {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "compact" || stored === "normal" || stored === "large") {
    return stored;
  }
  return DEFAULT_FONT_SIZE;
}

function applyFontSize(size: FontSize) {
  document.documentElement.setAttribute("data-font-size", size);
}

export function useFontSize() {
  const [fontSize, setFontSizeState] = useState<FontSize>(getStoredFontSize);

  useEffect(() => {
    applyFontSize(fontSize);
  }, [fontSize]);

  const setFontSize = useCallback((size: FontSize) => {
    setFontSizeState(size);
    localStorage.setItem(STORAGE_KEY, size);
  }, []);

  return { fontSize, setFontSize };
}
