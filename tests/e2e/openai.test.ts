import { describe, expect, it } from "vitest";
import { useE2EServer } from "./fake-upstream";

describe("OpenAI-compatible E2E", () => {
  const e2e = useE2EServer();

  it("serves models", async () => {
    const response = await fetch(`${e2e.api.baseUrl}/v1/models`);
    const body = await response.json();

    expect(body.data.some((model: { id: string }) => model.id === "gpt-5.5")).toBe(true);
  });

  it("serves chat completions through fake upstream", async () => {
    const response = await fetch(`${e2e.api.baseUrl}/v1/chat/completions`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        model: "gpt-5.5",
        messages: [{ role: "user", content: "hi" }],
      }),
    });
    const body = await response.json();

    expect(body.choices[0].message.content).toBe("hello");
  });

  it("serves responses through fake upstream", async () => {
    const response = await fetch(`${e2e.api.baseUrl}/v1/responses`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ input: "hi" }),
    });
    const body = await response.json();

    expect(body.id).toBe("resp_e2e");
  });
});
