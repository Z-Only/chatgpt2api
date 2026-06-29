import { describe, expect, it } from "vitest";
import { applyTheme, type ThemeTarget } from "./theme";

describe("applyTheme", () => {
  it("removes data-theme for system mode", () => {
    const target: ThemeTarget = { dataset: { theme: "dark" } };

    applyTheme("system", target);

    expect(target.dataset.theme).toBeUndefined();
  });

  it("sets explicit light and dark modes", () => {
    const target: ThemeTarget = { dataset: {} };

    applyTheme("light", target);
    expect(target.dataset.theme).toBe("light");

    applyTheme("dark", target);
    expect(target.dataset.theme).toBe("dark");
  });
});
