import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { verifyReleaseArtifacts } from "../scripts/verify-release-artifacts";

const roots: string[] = [];

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

const fixture = () => {
  const root = mkdtempSync(join(tmpdir(), "chatgpt2api-release-"));
  roots.push(root);
  const releaseDir = join(root, "release");
  mkdirSync(releaseDir, { recursive: true });

  const assets = [
    "ChatGPT2API_0.1.0_aarch64.dmg",
    "ChatGPT2API_0.1.0_x64.msi",
    "ChatGPT2API_0.1.0_x64-setup.exe",
    "ChatGPT2API_0.1.0_amd64.AppImage",
    "chatgpt2api_0.1.0_amd64.deb",
    "chatgpt2api-0.1.0-1.x86_64.rpm",
    "chatgpt2api-darwin-arm64.tar.gz",
    "chatgpt2api-darwin-x64.tar.gz",
    "chatgpt2api-linux-x64.tar.gz",
    "chatgpt2api-win32-x64.zip",
  ];

  for (const asset of assets) {
    writeFileSync(join(releaseDir, asset), asset);
  }

  const checksumLines = assets.map((asset) => `${"a".repeat(64)}  ${asset}`).join("\n");
  writeFileSync(join(releaseDir, "checksums-desktop-macOS.txt"), `${checksumLines}\n`);

  const npmPackagePath = join(root, "package.json");
  writeFileSync(npmPackagePath, JSON.stringify({ version: "0.1.0" }));

  const homebrewFormulaPath = join(root, "chatgpt2api.rb");
  writeFileSync(homebrewFormulaPath, 'class Chatgpt2api < Formula\n  version "0.1.0"\nend\n');

  return { homebrewFormulaPath, npmPackagePath, releaseDir };
};

describe("release artifact verifier", () => {
  it("accepts a complete release artifact set", () => {
    const paths = fixture();

    const result = verifyReleaseArtifacts({
      ...paths,
      tag: "v0.1.0",
    });

    expect(result.assetsChecked).toBe(10);
    expect(result.checksumFilesChecked).toBe(1);
  });

  it("rejects assets missing checksum coverage", () => {
    const paths = fixture();
    writeFileSync(
      join(paths.releaseDir, "checksums-desktop-macOS.txt"),
      `${"a".repeat(64)}  ChatGPT2API_0.1.0_aarch64.dmg\n`,
    );

    expect(() => verifyReleaseArtifacts({ ...paths, tag: "v0.1.0" })).toThrow(
      /Checksum entry missing/,
    );
  });

  it("rejects npm and Homebrew version mismatches", () => {
    const paths = fixture();
    writeFileSync(paths.npmPackagePath, JSON.stringify({ version: "0.2.0" }));

    expect(() => verifyReleaseArtifacts({ ...paths, tag: "v0.1.0" })).toThrow(
      /npm package version/,
    );

    writeFileSync(paths.npmPackagePath, JSON.stringify({ version: "0.1.0" }));
    writeFileSync(
      paths.homebrewFormulaPath,
      'class Chatgpt2api < Formula\n  version "0.2.0"\nend\n',
    );

    expect(() => verifyReleaseArtifacts({ ...paths, tag: "v0.1.0" })).toThrow(
      /Homebrew formula version/,
    );
  });
});
