const fs = require("fs");
const https = require("https");
const http = require("http");
const crypto = require("crypto");
const { mkdir, chmod, stat, rename, readFile, unlink, writeFile } = fs.promises;
const { createWriteStream } = fs;
const { pipeline } = require("stream/promises");
const path = require("path");

const {
  checksumManifestUrl,
  detectBinaryNames,
  releaseAssetUrl,
  releaseBinaryDirectory,
} = require("./artifacts");
const pkg = require("../package.json");

function resolvePackageVersion() {
  const configuredVersion =
    process.env.XIAOMIMIMO_TUI_VERSION ||
    process.env.XIAOMIMIMO_VERSION ||
    pkg.xiaomimimoBinaryVersion ||
    pkg.version;
  return String(configuredVersion).trim();
}

function resolveRepo() {
  return process.env.XIAOMIMIMO_TUI_GITHUB_REPO || process.env.XIAOMIMIMO_GITHUB_REPO || "xyuai/XiaomiMiMo-TUI";
}

function resolveTimeoutMs() {
  const configured =
    process.env.XIAOMIMIMO_TUI_DOWNLOAD_TIMEOUT_MS ||
    process.env.XIAOMIMIMO_DOWNLOAD_TIMEOUT_MS ||
    "30000";
  const parsed = Number.parseInt(String(configured).trim(), 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return 30000;
  }
  return Math.max(parsed, 1000);
}

function isOptionalInstall(argv = process.argv) {
  return argv.includes("--optional") || process.env.XIAOMIMIMO_TUI_OPTIONAL_INSTALL === "1";
}

function binaryPaths() {
  const { xiaomimimo, tui } = detectBinaryNames();
  const releaseDir = releaseBinaryDirectory();
  return {
    xiaomimimo: {
      asset: xiaomimimo,
      target: path.join(releaseDir, process.platform === "win32" ? "xiaomimimo.exe" : "xiaomimimo"),
    },
    tui: {
      asset: tui,
      target: path.join(releaseDir, process.platform === "win32" ? "xiaomimimo-tui.exe" : "xiaomimimo-tui"),
    },
  };
}

async function httpGet(url, timeoutMs = resolveTimeoutMs()) {
  const client = url.startsWith("https:") ? https : http;
  const response = await new Promise((resolve, reject) => {
    const request = client.get(url, (res) => {
      const status = res.statusCode || 0;
      if (status >= 300 && status < 400 && res.headers.location) {
        resolve({ redirect: res.headers.location, response: null });
        return;
      }
      if (status !== 200) {
        reject(new Error(`Request failed with status ${status}: ${url}`));
        return;
      }
      resolve({ redirect: null, response: res });
    });
    request.setTimeout(timeoutMs, () => {
      request.destroy(new Error(`Request timed out after ${timeoutMs}ms: ${url}`));
    });
    request.on("error", reject);
  });
  return response;
}

async function download(url, destination, timeoutMs = resolveTimeoutMs()) {
  const resolved = await httpGet(url, timeoutMs);
  if (resolved.redirect) {
    return download(resolved.redirect, destination, timeoutMs);
  }
  await mkdir(path.dirname(destination), { recursive: true });
  await pipeline(resolved.response, createWriteStream(destination));
}

async function downloadText(url, timeoutMs = resolveTimeoutMs()) {
  const resolved = await httpGet(url, timeoutMs);
  if (resolved.redirect) {
    return downloadText(resolved.redirect, timeoutMs);
  }
  const chunks = [];
  resolved.response.setEncoding("utf8");
  for await (const chunk of resolved.response) {
    chunks.push(chunk);
  }
  return chunks.join("");
}

async function readLocalVersion(file) {
  return readFile(file, "utf8").catch(() => "");
}

async function fileExists(file) {
  try {
    const result = await stat(file);
    return result.isFile();
  } catch {
    return false;
  }
}

function parseChecksumManifest(text) {
  const checksums = new Map();
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    const match = trimmed.match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (!match) {
      throw new Error(`Invalid checksum manifest line: ${trimmed}`);
    }
    checksums.set(match[2], match[1].toLowerCase());
  }
  return checksums;
}

async function sha256File(filePath) {
  const content = await readFile(filePath);
  return crypto.createHash("sha256").update(content).digest("hex");
}

async function verifyChecksum(filePath, assetName, checksums) {
  const expected = checksums.get(assetName);
  if (!expected) {
    throw new Error(`Checksum manifest is missing ${assetName}`);
  }
  const actual = await sha256File(filePath);
  if (actual !== expected) {
    throw new Error(
      `Checksum mismatch for ${assetName}: expected ${expected}, got ${actual}`,
    );
  }
}

async function loadChecksums(version, repo, timeoutMs) {
  return parseChecksumManifest(await downloadText(checksumManifestUrl(version, repo), timeoutMs));
}

async function ensureBinary(targetPath, assetName, version, repo, getChecksums, timeoutMs) {
  const marker = `${targetPath}.version`;
  const downloadIfNeeded =
    process.env.XIAOMIMIMO_TUI_FORCE_DOWNLOAD === "1" || process.env.XIAOMIMIMO_FORCE_DOWNLOAD === "1";
  if (!downloadIfNeeded) {
    const existing = await fileExists(targetPath);
    if (existing) {
      const markerVersion = await readLocalVersion(marker);
      if (markerVersion === String(version)) {
        return targetPath;
      }
    }
  }
  const checksums = await getChecksums();
  const url = releaseAssetUrl(assetName, version, repo);
  const destination = `${targetPath}.${process.pid}.${Date.now()}.download`;
  await download(url, destination, timeoutMs);
  try {
    await verifyChecksum(destination, assetName, checksums);
  } catch (error) {
    await unlink(destination).catch(() => {});
    throw error;
  }
  if (process.platform !== "win32") {
    await chmod(destination, 0o755);
  }
  await rename(destination, targetPath);
  await writeFile(marker, String(version), "utf8");
  return targetPath;
}

async function run() {
  if (process.env.XIAOMIMIMO_TUI_DISABLE_INSTALL === "1" || process.env.XIAOMIMIMO_DISABLE_INSTALL === "1") {
    return;
  }
  const version = resolvePackageVersion();
  const repo = resolveRepo();
  const paths = binaryPaths();
  const releaseDir = releaseBinaryDirectory();
  const timeoutMs = resolveTimeoutMs();
  await mkdir(releaseDir, { recursive: true });

  let checksumsPromise;
  const getChecksums = () => {
    if (!checksumsPromise) {
      checksumsPromise = loadChecksums(version, repo, timeoutMs);
    }
    return checksumsPromise;
  };

  await Promise.all([
    ensureBinary(paths.xiaomimimo.target, paths.xiaomimimo.asset, version, repo, getChecksums, timeoutMs),
    ensureBinary(paths.tui.target, paths.tui.asset, version, repo, getChecksums, timeoutMs),
  ]);
}

async function getBinaryPath(name) {
  await run();
  const paths = binaryPaths();
  if (name === "xiaomimimo") {
    return paths.xiaomimimo.target;
  }
  if (name === "xiaomimimo-tui") {
    return paths.tui.target;
  }
  throw new Error(`Unknown binary: ${name}`);
}

module.exports = {
  getBinaryPath,
  isOptionalInstall,
  resolveTimeoutMs,
  run,
};

if (require.main === module) {
  run().catch((error) => {
    if (isOptionalInstall()) {
      console.warn(`xiaomimimo-tui optional install skipped: ${error.message}`);
      console.warn("The package command will try again when first used, or you can rerun npm install.");
      process.exit(0);
    }
    console.error("xiaomimimo-tui install failed:", error.message);
    process.exit(1);
  });
}
