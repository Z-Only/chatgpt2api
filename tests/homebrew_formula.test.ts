import { describe, expect, it } from "vitest";
import { buildFormula } from "../packaging/homebrew/generate-formula";

describe("Homebrew formula generator", () => {
  it("renders a CLI-only formula from release checksums", () => {
    const formula = buildFormula({
      checksums: {
        darwinArm64: "a".repeat(64),
        darwinX64: "b".repeat(64),
        linuxX64: "c".repeat(64),
      },
      repository: "owner/chatgpt2api",
      version: "0.1.0",
    });

    expect(formula).toContain("class Chatgpt2api < Formula");
    expect(formula).toContain(
      'url "https://github.com/owner/chatgpt2api/releases/download/v0.1.0/chatgpt2api-darwin-arm64.tar.gz"',
    );
    expect(formula).toContain(`sha256 "${"a".repeat(64)}"`);
    expect(formula).toContain(
      'url "https://github.com/owner/chatgpt2api/releases/download/v0.1.0/chatgpt2api-darwin-x64.tar.gz"',
    );
    expect(formula).toContain(`sha256 "${"b".repeat(64)}"`);
    expect(formula).toContain(
      'url "https://github.com/owner/chatgpt2api/releases/download/v0.1.0/chatgpt2api-linux-x64.tar.gz"',
    );
    expect(formula).toContain(`sha256 "${"c".repeat(64)}"`);
    expect(formula).toContain('bin.install "chatgpt2api"');
    expect(formula).toContain(
      'assert_match "chatgpt2api", shell_output("#{bin}/chatgpt2api --help")',
    );
  });
});
