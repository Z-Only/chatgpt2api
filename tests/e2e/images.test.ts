import { describe, expect, it } from "vitest";
import { useE2EServer } from "./fake-upstream";

describe("Images E2E", () => {
  const e2e = useE2EServer();

  it("serves image generations through fake upstream", async () => {
    const response = await fetch(`${e2e.api.baseUrl}/v1/images/generations`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ prompt: "draw" }),
    });
    const body = await response.json();

    expect(body.data[0].b64_json).toBe("base64-image");
  });

  it("serves image edits through fake upstream", async () => {
    const form = new FormData();
    form.set("prompt", "edit");
    form.set("image", new Blob(["image-bytes"]), "input.png");

    const response = await fetch(`${e2e.api.baseUrl}/v1/images/edits`, {
      method: "POST",
      body: form,
    });
    const body = await response.json();

    expect(body.data[0].b64_json).toBe("base64-image");
  });
});
