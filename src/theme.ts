export type ThemeMode = "system" | "light" | "dark";
export interface ThemeTarget {
  dataset: {
    theme?: string;
  };
}

export function applyTheme(mode: ThemeMode, target: ThemeTarget = document.documentElement): void {
  if (mode === "system") {
    delete target.dataset.theme;
    return;
  }

  target.dataset.theme = mode;
}
