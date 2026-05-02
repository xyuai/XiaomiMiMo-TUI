const https = require("https");
const http = require("http");
const {
  allAssetNames,
  allReleaseAssetNames,
  checksumManifestUrl,
  legacyDuplicateAssetNames,
  releaseAssetUrl,
} = require("./artifacts");

const pkg = require("../package.json");

function resolveBinaryVersion() {
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

function resolveGitHubToken() {
  return process.env.GITHUB_TOKEN || process.env.GH_TOKEN || "";
}

function githubApiBase(repo = "xyuai/XiaomiMiMo-TUI") {
  const override =
    process.env.XIAOMIMIMO_TUI_GITHUB_API_URL || process.env.XIAOMIMIMO_GITHUB_API_URL;
  if (override) {
    const trimmed = String(override).trim().replace(/\/+$/, "");
    return `${trimmed}/repos/${repo}`;
  }
  return `https://api.github.com/repos/${repo}`;
}

function requestHeaders(extra = {}) {
  const token = resolveGitHubToken();
  return {
    "User-Agent": "xiaomimimo-tui-npm-release-check",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...extra,
  };
}

function requestStatus(url, method = "HEAD", redirects = 0) {
  if (redirects > 10) {
    throw new Error(`Too many redirects while checking ${url}`);
  }
  const client = url.startsWith("https:") ? https : http;
  return new Promise((resolve, reject) => {
    const req = client.request(
      url,
      {
        method,
        headers: requestHeaders(),
      },
      (res) => {
        const status = res.statusCode || 0;
        const location = res.headers.location;
        res.resume();
        if (status >= 300 && status < 400 && location) {
          const next = new URL(location, url).toString();
          resolve(requestStatus(next, method, redirects + 1));
          return;
        }
        resolve(status);
      },
    );
    req.on("error", reject);
    req.end();
  });
}

async function withRetries(label, operation, attempts = 3) {
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      if (attempt === attempts) {
        break;
      }
      const delayMs = 500 * attempt;
      console.warn(`  retry ${attempt}/${attempts - 1} for ${label}: ${error.message}`);
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }
  throw lastError;
}

async function verifyAsset(url, label) {
  let status = await withRetries(`${label} HEAD`, () => requestStatus(url, "HEAD"));
  if (status === 403 || status === 405) {
    status = await withRetries(`${label} GET`, () => requestStatus(url, "GET"));
  }
  if (status < 200 || status >= 400) {
    throw new Error(`${label} returned HTTP ${status} (${url})`);
  }
}

async function verifyAssetMissing(url, label) {
  let status = await withRetries(`${label} HEAD`, () => requestStatus(url, "HEAD"));
  if (status === 403 || status === 405) {
    status = await withRetries(`${label} GET`, () => requestStatus(url, "GET"));
  }
  if (status >= 200 && status < 400) {
    throw new Error(`${label} should not be published because it duplicates the platform-qualified asset (${url})`);
  }
}

async function downloadText(url) {
  const client = url.startsWith("https:") ? https : http;
  return new Promise((resolve, reject) => {
    client
      .get(
        url,
        {
          headers: requestHeaders(),
        },
        (res) => {
          const status = res.statusCode || 0;
          if (status >= 300 && status < 400 && res.headers.location) {
            const next = new URL(res.headers.location, url).toString();
            resolve(downloadText(next));
            return;
          }
          if (status !== 200) {
            reject(new Error(`Request failed with status ${status}: ${url}`));
            res.resume();
            return;
          }
          const chunks = [];
          res.setEncoding("utf8");
          res.on("data", (chunk) => chunks.push(chunk));
          res.on("end", () => resolve(chunks.join("")));
        },
      )
      .on("error", reject);
  });
}

async function downloadJson(url) {
  const text = await downloadText(url);
  return JSON.parse(text);
}

async function fetchReleaseAssetIndex(repo, version) {
  const tag = `v${version}`;
  const encodedTag = encodeURIComponent(tag);
  const url = `${githubApiBase(repo)}/releases/tags/${encodedTag}`;
  return withRetries(`GitHub release API ${tag}`, () => downloadJson(url));
}

function findApiAsset(release, name) {
  return (release.assets || []).find((asset) => asset.name === name);
}

function verifyAssetsFromApi(release, expectedAssets, forbiddenAssets) {
  const errors = [];
  for (const asset of expectedAssets) {
    if (!findApiAsset(release, asset)) {
      errors.push(`missing ${asset}`);
    }
  }
  for (const asset of forbiddenAssets) {
    if (findApiAsset(release, asset)) {
      errors.push(`forbidden duplicate ${asset} is still published`);
    }
  }
  if (errors.length > 0) {
    throw new Error(`Release API asset check failed: ${errors.join("; ")}`);
  }
}

function checksumsFromReleaseApi(release, expectedAssets) {
  const checksums = new Map();
  const hashes = new Map();
  for (const assetName of expectedAssets) {
    if (assetName === "xiaomimimo-artifacts-sha256.txt") {
      continue;
    }
    const asset = findApiAsset(release, assetName);
    const digest = asset?.digest || "";
    const match = digest.match(/^sha256:([a-fA-F0-9]{64})$/);
    if (!match) {
      throw new Error(`GitHub API did not return a sha256 digest for ${assetName}`);
    }
    const hash = match[1].toLowerCase();
    const duplicate = hashes.get(hash);
    if (duplicate) {
      throw new Error(`GitHub API reports duplicate asset hashes: ${duplicate} = ${assetName}`);
    }
    hashes.set(hash, assetName);
    checksums.set(assetName, hash);
  }
  return checksums;
}

function parseChecksumManifest(text) {
  const checksums = new Map();
  const duplicateNames = [];
  const duplicateHashes = new Map();
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    const match = trimmed.match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (!match) {
      throw new Error(`Invalid checksum manifest line: ${trimmed}`);
    }
    const name = match[2];
    const hash = match[1].toLowerCase();
    if (checksums.has(name)) {
      duplicateNames.push(name);
    }
    const namesForHash = duplicateHashes.get(hash) || [];
    namesForHash.push(name);
    duplicateHashes.set(hash, namesForHash);
    checksums.set(name, hash);
  }
  if (duplicateNames.length > 0) {
    throw new Error(`Checksum manifest contains duplicate asset names: ${duplicateNames.join(", ")}`);
  }
  const duplicateAssetGroups = Array.from(duplicateHashes.values()).filter((names) => names.length > 1);
  if (duplicateAssetGroups.length > 0) {
    throw new Error(`Checksum manifest contains duplicate asset hashes: ${duplicateAssetGroups.map((names) => names.join(" = ")).join("; ")}`);
  }
  return checksums;
}

async function run() {
  const version = resolveBinaryVersion();
  const repo = resolveRepo();
  const assets = allReleaseAssetNames();
  const legacyAssets = legacyDuplicateAssetNames();
  let release = null;

  try {
    release = await fetchReleaseAssetIndex(repo, version);
    verifyAssetsFromApi(release, assets, legacyAssets);
    console.log(`GitHub API asset index ok for ${repo}@v${version}.`);
  } catch (error) {
    console.warn(`GitHub API asset index unavailable: ${error.message}`);
  }

  console.log(`Verifying ${assets.length} release assets for ${repo}@v${version}...`);
  for (const asset of assets) {
    if (release && findApiAsset(release, asset)) {
      console.log(`  ok ${asset}`);
      continue;
    }
    const url = releaseAssetUrl(asset, version, repo);
    await verifyAsset(url, asset);
    console.log(`  ok ${asset}`);
  }
  for (const asset of legacyAssets) {
    if (release && !findApiAsset(release, asset)) {
      console.log(`  absent ${asset}`);
      continue;
    }
    const url = releaseAssetUrl(asset, version, repo);
    await verifyAssetMissing(url, asset);
    console.log(`  absent ${asset}`);
  }
  const manifestAsset = release ? findApiAsset(release, "xiaomimimo-artifacts-sha256.txt") : null;
  const manifestUrl = manifestAsset?.browser_download_url || checksumManifestUrl(version, repo);
  let checksums;
  try {
    checksums = parseChecksumManifest(
      await withRetries("checksum manifest download", () => downloadText(manifestUrl)),
    );
  } catch (error) {
    if (!release) {
      throw error;
    }
    console.warn(
      `  checksum manifest download unavailable (${error.message}); falling back to GitHub API asset digests.`,
    );
    checksums = checksumsFromReleaseApi(release, assets);
  }
  for (const asset of allAssetNames()) {
    if (!checksums.has(asset)) {
      throw new Error(`Checksum manifest is missing ${asset}`);
    }
  }
  console.log("Release assets verified.");
}

run().catch((error) => {
  console.error("Release asset verification failed:", error.message);
  process.exit(1);
});
