import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

type FormulaInput = {
  checksums: {
    darwinArm64: string;
    darwinX64: string;
    linuxX64: string;
  };
  repository: string;
  version: string;
};

const normalizeVersion = (version: string) => version.replace(/^v/, "");

const assetUrl = (repository: string, version: string, asset: string) =>
  `https://github.com/${repository}/releases/download/v${normalizeVersion(version)}/${asset}`;

export const buildFormula = ({ checksums, repository, version }: FormulaInput) => {
  const normalizedVersion = normalizeVersion(version);

  return `class Chatgpt2api < Formula
  desc "Local OpenAI-compatible API bridge for ChatGPT/Codex"
  homepage "https://github.com/${repository}"
  version "${normalizedVersion}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "${assetUrl(repository, normalizedVersion, "chatgpt2api-darwin-arm64.tar.gz")}"
      sha256 "${checksums.darwinArm64}"
    else
      url "${assetUrl(repository, normalizedVersion, "chatgpt2api-darwin-x64.tar.gz")}"
      sha256 "${checksums.darwinX64}"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "${assetUrl(repository, normalizedVersion, "chatgpt2api-linux-x64.tar.gz")}"
      sha256 "${checksums.linuxX64}"
    end
  end

  def install
    bin.install "chatgpt2api"
  end

  test do
    assert_match "chatgpt2api", shell_output("#{bin}/chatgpt2api --help")
  end
end
`;
};

const required = (name: string) => {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
};

export const main = () => {
  const owner = required("GITHUB_REPOSITORY_OWNER");
  const repository = process.env.GITHUB_REPOSITORY ?? `${owner}/chatgpt2api`;
  const version = required("RELEASE_VERSION");
  const formula = buildFormula({
    checksums: {
      darwinArm64: required("CHATGPT2API_DARWIN_ARM64_SHA256"),
      darwinX64: required("CHATGPT2API_DARWIN_X64_SHA256"),
      linuxX64: required("CHATGPT2API_LINUX_X64_SHA256"),
    },
    repository,
    version,
  });

  mkdirSync("packaging/homebrew", { recursive: true });
  writeFileSync("packaging/homebrew/chatgpt2api.rb", formula);
};

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main();
}
