import { describe, expect, it } from "vitest";
import { resolveSystemLocale } from "./locale";

describe("resolveSystemLocale", () => {
  it("uses Chinese for zh-prefixed locales", () => {
    expect(resolveSystemLocale(["zh-CN"])).toBe("zh");
  });

  it("uses English for other locales", () => {
    expect(resolveSystemLocale(["ja-JP"])).toBe("en");
  });
});
