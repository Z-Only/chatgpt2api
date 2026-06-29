import { describe, expect, it } from "vitest";
import { useE2EServer } from "./fake-upstream";

describe("Responses WebSocket E2E", () => {
  const e2e = useE2EServer();

  it("forwards response.create and receives terminal event", async () => {
    const event = await new Promise<string>((resolve, reject) => {
      const socket = new WebSocket(`ws://${new URL(e2e.api.baseUrl).host}/v1/responses`);
      socket.addEventListener("open", () => {
        socket.send(JSON.stringify({ type: "response.create", response: { input: "hi" } }));
      });
      socket.addEventListener("message", (message) => {
        resolve(String(message.data));
        socket.close();
      });
      socket.addEventListener("error", () => reject(new Error("websocket failed")));
    });

    expect(event).toContain("response.completed");
  });
});
