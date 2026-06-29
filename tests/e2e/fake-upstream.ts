import { createHash } from "node:crypto";
import { createServer, type IncomingMessage } from "node:http";
import { createServer as createNetServer } from "node:net";
import { spawn, type ChildProcess } from "node:child_process";
import type { Duplex } from "node:stream";
import { afterAll, beforeAll } from "vitest";

export interface FakeUpstream {
  url: string;
  wsUrl: string;
  requests: unknown[];
  stop: () => void;
}

export interface ApiProcess {
  baseUrl: string;
  stop: () => void;
}

export async function startFakeUpstream(): Promise<FakeUpstream> {
  const requests: unknown[] = [];
  const server = createServer(async (request, response) => {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    if (request.method === "POST" && url.pathname === "/responses") {
      requests.push(JSON.parse(await readBody(request)));
      response.setHeader("content-type", "application/json");
      response.end(
        JSON.stringify({
          id: "resp_e2e",
          output: [{ type: "message", content: [{ type: "output_text", text: "hello" }] }],
          usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2 },
        }),
      );
      return;
    }
    if (
      request.method === "POST" &&
      (url.pathname === "/images/generations" || url.pathname === "/images/edits")
    ) {
      requests.push({ path: url.pathname, body: await readBody(request) });
      response.setHeader("content-type", "application/json");
      response.end(JSON.stringify({ data: [{ b64_json: "base64-image" }] }));
      return;
    }
    response.statusCode = 404;
    response.end();
  });
  server.on("upgrade", (request, socket) => {
    acceptWebSocket(request, socket);
    socket.once("data", (frame) => {
      requests.push(JSON.parse(readWebSocketText(frame)));
      writeWebSocketText(socket, JSON.stringify({ type: "response.completed" }));
      socket.end();
    });
  });

  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("fake upstream address unavailable");
  }

  return {
    url: `http://127.0.0.1:${address.port}`,
    wsUrl: `ws://127.0.0.1:${address.port}/ws`,
    requests,
    stop: () => server.close(),
  };
}

export async function freePort(): Promise<number> {
  return await new Promise((resolve, reject) => {
    const server = createNetServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (typeof address === "object" && address) {
        const port = address.port;
        server.close(() => resolve(port));
        return;
      }
      reject(new Error("port unavailable"));
    });
  });
}

export async function startApiProcess(fake: FakeUpstream): Promise<ApiProcess> {
  const port = await freePort();
  const proc = spawn(
    "cargo",
    ["run", "--manifest-path", "src-tauri/Cargo.toml", "--", "serve", "--port", String(port)],
    {
      env: {
        ...process.env,
        CHATGPT2API_FAKE_UPSTREAM_BASE_URL: fake.url,
        CHATGPT2API_FAKE_UPSTREAM_WS_URL: fake.wsUrl,
      },
      stdio: "ignore",
    },
  );
  const baseUrl = `http://127.0.0.1:${port}`;
  await waitForHealth(baseUrl);

  return {
    baseUrl,
    stop: () => stopProcess(proc),
  };
}

export async function waitForHealth(baseUrl: string): Promise<void> {
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${baseUrl}/health`);
      if (response.ok) {
        return;
      }
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
  }
  throw new Error("server did not become healthy");
}

export function useE2EServer() {
  let fake: FakeUpstream | undefined;
  let api: ApiProcess | undefined;

  beforeAll(async () => {
    fake = await startFakeUpstream();
    api = await startApiProcess(fake);
  }, 30_000);

  afterAll(() => {
    api?.stop();
    fake?.stop();
  });

  return {
    get fake() {
      if (!fake) {
        throw new Error("fake upstream not started");
      }
      return fake;
    },
    get api() {
      if (!api) {
        throw new Error("api process not started");
      }
      return api;
    },
  };
}

async function readBody(request: IncomingMessage): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of request) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString("utf8");
}

function acceptWebSocket(request: IncomingMessage, socket: Duplex): void {
  const key = request.headers["sec-websocket-key"];
  if (typeof key !== "string") {
    socket.destroy();
    return;
  }
  const accept = createHash("sha1")
    .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
    .digest("base64");
  socket.write(
    [
      "HTTP/1.1 101 Switching Protocols",
      "Upgrade: websocket",
      "Connection: Upgrade",
      `Sec-WebSocket-Accept: ${accept}`,
      "\r\n",
    ].join("\r\n"),
  );
}

function readWebSocketText(frame: Buffer): string {
  let offset = 2;
  let length = frame[1] & 0x7f;
  if (length === 126) {
    length = frame.readUInt16BE(offset);
    offset += 2;
  }
  const masked = (frame[1] & 0x80) !== 0;
  const mask = masked ? frame.subarray(offset, offset + 4) : Buffer.alloc(0);
  offset += masked ? 4 : 0;
  const payload = frame.subarray(offset, offset + length);
  if (!masked) {
    return payload.toString("utf8");
  }
  return Buffer.from(payload.map((byte, index) => byte ^ mask[index % 4])).toString("utf8");
}

function writeWebSocketText(socket: Duplex, text: string): void {
  const payload = Buffer.from(text);
  if (payload.length >= 126) {
    throw new Error("test websocket payload too large");
  }
  socket.write(Buffer.concat([Buffer.from([0x81, payload.length]), payload]));
}

function stopProcess(process: ChildProcess): void {
  if (!process.killed) {
    process.kill();
  }
}
