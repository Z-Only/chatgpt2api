import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";

type VerifyOptions = {
  homebrewFormulaPath?: string;
  npmPackagePath?: string;
  releaseDir?: string;
  tag: string;
};

type VerifyResult = {
  assetsChecked: number;
  checksumFilesChecked: number;
};

const cliAssets = [
  "chatgpt2api-darwin-arm64.tar.gz",
  "chatgpt2api-darwin-x64.tar.gz",
  "chatgpt2api-linux-x64.tar.gz",
  "chatgpt2api-win32-x64.zip",
];

const desktopRequirements = [
  { label: "macOS DMG", matches: (name: string) => name.endsWith(".dmg") },
  { label: "Windows MSI", matches: (name: string) => name.endsWith(".msi") },
  { label: "Windows NSIS", matches: (name: string) => name.endsWith(".exe") },
  { label: "Linux AppImage", matches: (name: string) => name.endsWith(".AppImage") },
  { label: "Linux deb", matches: (name: string) => name.endsWith(".deb") },
  { label: "Linux rpm", matches: (name: string) => name.endsWith(".rpm") },
];

const tagVersion = (tag: string) => tag.replace(/^v/, "");

const listFiles = (directory: string): string[] => {
  const entries = readdirSync(directory, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return listFiles(path);
    }
    return [path];
  });
};

const parseChecksumEntries = (files: string[]) => {
  const entries = new Set<string>();
  for (const file of files) {
    for (const line of readFileSync(file, "utf8").split(/\r?\n/)) {
      const match = line.trim().match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
      if (match) {
        entries.add(basename(match[2].trim()));
      }
    }
  }
  return entries;
};

const readPackageVersion = (packagePath: string) => {
  const parsed = JSON.parse(readFileSync(packagePath, "utf8")) as { version?: string };
  if (!parsed.version) {
    throw new Error(`${packagePath} does not declare a version.`);
  }
  return parsed.version;
};

const readFormulaVersion = (formulaPath: string) => {
  const match = readFileSync(formulaPath, "utf8").match(/^\s*version\s+"([^"]+)"/m);
  if (!match) {
    throw new Error(`${formulaPath} does not declare a Homebrew version.`);
  }
  return match[1];
};

export const verifyReleaseArtifacts = ({
  homebrewFormulaPath = "packaging/homebrew/chatgpt2api.rb",
  npmPackagePath = "npm/chatgpt2api/package.json",
  releaseDir = "release",
  tag,
}: VerifyOptions): VerifyResult => {
  if (!tag) {
    throw new Error("--tag is required.");
  }
  if (!existsSync(releaseDir) || !statSync(releaseDir).isDirectory()) {
    throw new Error(`Release artifact directory not found: ${releaseDir}`);
  }

  const version = tagVersion(tag);
  const files = listFiles(releaseDir);
  const checksumFiles = files.filter((file) => /^checksums.*\.txt$/.test(basename(file)));
  if (checksumFiles.length === 0) {
    throw new Error(`No checksum files found in ${releaseDir}.`);
  }

  const releaseAssets = files
    .filter((file) => !checksumFiles.includes(file))
    .map((file) => basename(file));
  const assetNames = new Set(releaseAssets);

  for (const asset of cliAssets) {
    if (!assetNames.has(asset)) {
      throw new Error(`Missing CLI release asset: ${asset}`);
    }
  }

  for (const requirement of desktopRequirements) {
    if (!releaseAssets.some(requirement.matches)) {
      throw new Error(`Missing desktop release asset for ${requirement.label}.`);
    }
  }

  const checksumEntries = parseChecksumEntries(checksumFiles);
  for (const asset of releaseAssets) {
    if (!checksumEntries.has(asset)) {
      throw new Error(`Checksum entry missing for release asset: ${asset}`);
    }
  }

  const npmVersion = readPackageVersion(npmPackagePath);
  if (npmVersion !== version) {
    throw new Error(`npm package version ${npmVersion} does not match tag ${tag}.`);
  }

  const formulaVersion = readFormulaVersion(homebrewFormulaPath);
  if (formulaVersion !== version) {
    throw new Error(`Homebrew formula version ${formulaVersion} does not match tag ${tag}.`);
  }

  return {
    assetsChecked: releaseAssets.length,
    checksumFilesChecked: checksumFiles.length,
  };
};

const parseArgs = (args: string[]): VerifyOptions => {
  const options: Partial<VerifyOptions> = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    const value = args[index + 1];
    if (arg === "--tag") {
      options.tag = value;
      index += 1;
    } else if (arg === "--release-dir") {
      options.releaseDir = value;
      index += 1;
    } else if (arg === "--npm-package") {
      options.npmPackagePath = value;
      index += 1;
    } else if (arg === "--homebrew-formula") {
      options.homebrewFormulaPath = value;
      index += 1;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  if (!options.tag) {
    throw new Error("Usage: bun run scripts/verify-release-artifacts.ts --tag v0.1.0");
  }
  return options as VerifyOptions;
};

export const main = () => {
  const result = verifyReleaseArtifacts(parseArgs(process.argv.slice(2)));
  console.log(
    `Verified ${result.assetsChecked} release assets across ${result.checksumFilesChecked} checksum file(s).`,
  );
};

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main();
}
