const { createHash } = require("node:crypto");
const {
  chmodSync,
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} = require("node:fs");
const http = require("node:http");
const https = require("node:https");
const os = require("node:os");
const { basename, dirname, join, resolve } = require("node:path");
const { spawnSync } = require("node:child_process");
const { pipeline } = require("node:stream/promises");
const { URL } = require("node:url");

const defaultPackageRoot = resolve(__dirname, "..");
const packageJson = require("../package.json");

const supportedTargets = {
  "darwin-arm64": {
    assetName: "chatgpt2api-darwin-arm64.tar.gz",
    binaryName: "chatgpt2api",
    checksumName: "checksums-cli-macOS.txt",
  },
  "darwin-x64": {
    assetName: "chatgpt2api-darwin-x64.tar.gz",
    binaryName: "chatgpt2api",
    checksumName: "checksums-cli-macOS.txt",
  },
  "linux-x64": {
    assetName: "chatgpt2api-linux-x64.tar.gz",
    binaryName: "chatgpt2api",
    checksumName: "checksums-cli-Linux.txt",
  },
  "win32-x64": {
    assetName: "chatgpt2api-win32-x64.zip",
    binaryName: "chatgpt2api.exe",
    checksumName: "checksums-cli-Windows.txt",
  },
};

const trimTrailingSlash = (value) => value.replace(/\/+$/, "");
const normalVersion = (version) => String(version).replace(/^v/, "");

const supportedTargetList = () => Object.keys(supportedTargets).sort().join(", ");

const getPlatformTarget = (platform = process.platform, arch = process.arch) => {
  const key = `${platform}-${arch}`;
  const target = supportedTargets[key];
  if (!target) {
    throw new Error(
      `Unsupported platform for chatgpt2api CLI: ${key}. Supported release targets: ${supportedTargetList()}.`,
    );
  }
  return { ...target, arch, key, platform };
};

const binaryPath = ({
  packageRoot = defaultPackageRoot,
  version = packageJson.version,
  platform = process.platform,
  arch = process.arch,
} = {}) => {
  const target = getPlatformTarget(platform, arch);
  return join(packageRoot, ".cache", `v${normalVersion(version)}`, target.key, target.binaryName);
};

const parseRepository = (repository) => {
  const value = typeof repository === "string" ? repository : repository?.url;
  if (!value) {
    return undefined;
  }
  const match = value.match(/github\.com[:/]([^/\s]+\/[^/\s#]+?)(?:\.git)?(?:[#\s]|$)/);
  return match?.[1];
};

const releaseBaseUrl = ({ env = process.env, pkg = packageJson, version = pkg.version } = {}) => {
  if (env.CHATGPT2API_RELEASE_BASE_URL) {
    return trimTrailingSlash(env.CHATGPT2API_RELEASE_BASE_URL);
  }

  const configuredBase = pkg.chatgpt2api?.releaseBaseUrl;
  if (configuredBase) {
    return trimTrailingSlash(configuredBase.replaceAll("{version}", normalVersion(version)));
  }

  const repository =
    env.CHATGPT2API_RELEASE_REPOSITORY ||
    pkg.chatgpt2api?.releaseRepository ||
    parseRepository(pkg.repository) ||
    "chatgpt2api/chatgpt2api";

  return `https://github.com/${repository}/releases/download/v${normalVersion(version)}`;
};

const download = async (url, destination, redirects = 0) => {
  if (redirects > 5) {
    throw new Error(`Too many redirects while downloading ${url}`);
  }

  const parsed = new URL(url);
  const client = parsed.protocol === "http:" ? http : https;

  await new Promise((resolveDownload, rejectDownload) => {
    const request = client.get(parsed, async (response) => {
      const status = response.statusCode ?? 0;
      if (status >= 300 && status < 400 && response.headers.location) {
        response.resume();
        try {
          await download(
            new URL(response.headers.location, parsed).toString(),
            destination,
            redirects + 1,
          );
          resolveDownload();
        } catch (error) {
          rejectDownload(error);
        }
        return;
      }

      if (status < 200 || status >= 300) {
        response.resume();
        rejectDownload(new Error(`Download failed for ${url}: HTTP ${status}`));
        return;
      }

      try {
        await pipeline(response, createWriteStream(destination));
        resolveDownload();
      } catch (error) {
        rejectDownload(error);
      }
    });

    request.on("error", rejectDownload);
  });
};

const downloadText = async (url, destination) => {
  await download(url, destination);
  return readFileSync(destination, "utf8");
};

const sha256File = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");

const extractExpectedChecksum = (checksumText, assetName) => {
  for (const line of checksumText.split(/\r?\n/)) {
    const match = line.trim().match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (match && basename(match[2].trim()) === assetName) {
      return match[1].toLowerCase();
    }
  }
  throw new Error(`Checksum for ${assetName} was not found in the release checksum file.`);
};

const verifyChecksum = (file, expectedChecksum) => {
  const actual = sha256File(file);
  if (actual !== expectedChecksum.toLowerCase()) {
    throw new Error(
      `Checksum mismatch for ${basename(file)}: expected ${expectedChecksum}, got ${actual}`,
    );
  }
};

const runArchiveTool = (command, args) => {
  const result = spawnSync(command, args, { encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error((result.stderr || result.stdout || `${command} failed`).trim());
  }
};

const extractArchive = (archivePath, extractRoot, target) => {
  mkdirSync(extractRoot, { recursive: true });
  try {
    runArchiveTool("tar", ["-xf", archivePath, "-C", extractRoot]);
  } catch (tarError) {
    if (!target.assetName.endsWith(".zip")) {
      throw tarError;
    }
    runArchiveTool("powershell.exe", [
      "-NoProfile",
      "-Command",
      `Expand-Archive -LiteralPath '${archivePath.replaceAll("'", "''")}' -DestinationPath '${extractRoot.replaceAll(
        "'",
        "''",
      )}' -Force`,
    ]);
  }
};

const findBinary = (root, binaryName) => {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const candidate = join(root, entry.name);
    if (entry.isFile() && entry.name === binaryName) {
      return candidate;
    }
    if (entry.isDirectory()) {
      const nested = findBinary(candidate, binaryName);
      if (nested) {
        return nested;
      }
    }
  }
  return undefined;
};

const installBinary = async ({
  packageRoot = defaultPackageRoot,
  version = packageJson.version,
  platform = process.platform,
  arch = process.arch,
  releaseBaseUrl: configuredReleaseBaseUrl,
} = {}) => {
  const versionWithoutPrefix = normalVersion(version);
  const target = getPlatformTarget(platform, arch);
  const destination = binaryPath({ packageRoot, version: versionWithoutPrefix, platform, arch });
  const marker = join(dirname(destination), "version");

  if (
    existsSync(destination) &&
    existsSync(marker) &&
    readFileSync(marker, "utf8").trim() === versionWithoutPrefix
  ) {
    return destination;
  }

  const baseUrl = trimTrailingSlash(
    configuredReleaseBaseUrl ?? releaseBaseUrl({ version: versionWithoutPrefix, pkg: packageJson }),
  );
  const tempRoot = mkdtempSync(join(os.tmpdir(), "chatgpt2api-install-"));
  const archivePath = join(tempRoot, target.assetName);
  const checksumPath = join(tempRoot, target.checksumName);
  const extractRoot = join(tempRoot, "extract");

  try {
    await download(`${baseUrl}/${target.assetName}`, archivePath);
    const checksumText = await downloadText(`${baseUrl}/${target.checksumName}`, checksumPath);
    verifyChecksum(archivePath, extractExpectedChecksum(checksumText, target.assetName));
    extractArchive(archivePath, extractRoot, target);

    const extractedBinary = findBinary(extractRoot, target.binaryName);
    if (!extractedBinary || !statSync(extractedBinary).isFile()) {
      throw new Error(`${target.binaryName} was not found in ${target.assetName}`);
    }

    mkdirSync(dirname(destination), { recursive: true });
    copyFileSync(extractedBinary, destination);
    if (platform !== "win32") {
      chmodSync(destination, 0o755);
    }
    writeFileSync(marker, `${versionWithoutPrefix}\n`);
    return destination;
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
};

const main = async () => {
  try {
    const installed = await installBinary();
    console.log(`Installed chatgpt2api CLI binary to ${installed}`);
  } catch (error) {
    console.error("Failed to install chatgpt2api CLI binary.");
    console.error(error.message);
    console.error(
      "Set CHATGPT2API_RELEASE_BASE_URL to a release asset directory or install from GitHub Releases manually.",
    );
    process.exit(1);
  }
};

module.exports = {
  binaryPath,
  extractExpectedChecksum,
  getPlatformTarget,
  installBinary,
  releaseBaseUrl,
};

if (require.main === module) {
  void main();
}
