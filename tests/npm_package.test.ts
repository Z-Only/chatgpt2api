import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { createServer, type Server } from "node:http";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { afterEach, describe, expect, it } from "vitest";

const require = createRequire(import.meta.url);
const installer = require("../npm/chatgpt2api/scripts/postinstall.js");

const tempRoots: string[] = [];
const servers: Server[] = [];

afterEach(async () => {
  await Promise.all(
    servers.splice(0).map(
      (server) =>
        new Promise<void>((resolve, reject) => {
          server.close((error) => (error ? reject(error) : resolve()));
        }),
    ),
  );
  for (const root of tempRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

const tempRoot = () => {
  const root = mkdtempSync(join(tmpdir(), "chatgpt2api-npm-"));
  tempRoots.push(root);
  return root;
};

describe("npm CLI distribution", () => {
  it("maps supported platforms to release assets", () => {
    expect(installer.getPlatformTarget("darwin", "arm64")).toMatchObject({
      assetName: "chatgpt2api-darwin-arm64.tar.gz",
      checksumName: "checksums-macOS.txt",
      binaryName: "chatgpt2api",
    });
    expect(installer.getPlatformTarget("win32", "x64")).toMatchObject({
      assetName: "chatgpt2api-win32-x64.zip",
      checksumName: "checksums-Windows.txt",
      binaryName: "chatgpt2api.exe",
    });
    expect(() => installer.getPlatformTarget("linux", "arm64")).toThrow(/Unsupported platform/);
  });

  it("extracts an asset checksum from release checksum files", () => {
    const checksum = installer.extractExpectedChecksum(
      [
        "0".repeat(64) + "  artifacts/chatgpt2api-darwin-x64.tar.gz",
        "a".repeat(64) + "  artifacts/chatgpt2api-darwin-arm64.tar.gz",
      ].join("\n"),
      "chatgpt2api-darwin-arm64.tar.gz",
    );

    expect(checksum).toBe("a".repeat(64));
  });

  it("downloads, verifies, extracts, and caches the CLI binary", async () => {
    const root = tempRoot();
    const releaseRoot = join(root, "release");
    const payloadRoot = join(root, "payload");
    const packageRoot = join(root, "package");
    const version = "0.1.0";
    const target = installer.getPlatformTarget("darwin", "arm64");

    mkdirSync(releaseRoot, { recursive: true });
    mkdirSync(payloadRoot, { recursive: true });
    writeFileSync(join(payloadRoot, target.binaryName), "#!/bin/sh\necho chatgpt2api\n");

    const archivePath = join(releaseRoot, target.assetName);
    const tar = spawnSync("tar", ["-czf", archivePath, "-C", payloadRoot, target.binaryName]);
    expect(tar.status).toBe(0);

    const sha256 = createHash("sha256").update(readFileSync(archivePath)).digest("hex");
    writeFileSync(
      join(releaseRoot, target.checksumName),
      `${sha256}  artifacts/${target.assetName}\n`,
    );

    const server = createServer((request, response) => {
      const requested = basename(request.url ?? "");
      const file = join(releaseRoot, requested);
      response.writeHead(200);
      response.end(readFileSync(file));
    });
    servers.push(server);
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    if (!address || typeof address === "string") {
      throw new Error("Expected local test server port");
    }

    const binaryPath = await installer.installBinary({
      packageRoot,
      version,
      platform: "darwin",
      arch: "arm64",
      releaseBaseUrl: `http://127.0.0.1:${address.port}/v${version}`,
    });

    expect(binaryPath).toBe(
      join(packageRoot, ".cache", `v${version}`, "darwin-arm64", target.binaryName),
    );
    expect(readFileSync(binaryPath, "utf8")).toContain("chatgpt2api");
  });
});
