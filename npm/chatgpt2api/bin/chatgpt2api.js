#!/usr/bin/env node

const { spawn } = require("node:child_process");
const { existsSync } = require("node:fs");
const { binaryPath, getPlatformTarget } = require("../scripts/postinstall.js");

const fail = (message) => {
  console.error(message);
  process.exit(1);
};

let executable;
try {
  getPlatformTarget();
  executable = binaryPath();
} catch (error) {
  fail(error.message);
}

if (!existsSync(executable)) {
  fail(
    [
      "chatgpt2api CLI binary is not installed.",
      "Reinstall the npm package so the postinstall downloader can fetch the release binary:",
      "  npm install -g chatgpt2api",
    ].join("\n"),
  );
}

const child = spawn(executable, process.argv.slice(2), { stdio: "inherit" });

child.on("error", (error) => {
  fail(`Failed to start chatgpt2api CLI: ${error.message}`);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
