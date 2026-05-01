const path = require("path");
const os = require("os");

const CHECKSUM_MANIFEST = "xiaomimimo-artifacts-sha256.txt";

const ASSET_MATRIX = {
  win32: {
    x64: ["xiaomimimo-windows-x64.exe", "xiaomimimo-tui-windows-x64.exe"],
  },
};

function detectBinaryNames() {
  const platform = os.platform();
  const arch = os.arch();
  const defaults = ASSET_MATRIX[platform];
  if (!defaults) {
    const supported = Object.keys(ASSET_MATRIX).map(p => `'${p}'`).join(', ');
    throw new Error(`Unsupported platform: ${platform}. Supported platforms: ${supported}`);
  }
  const pair = defaults[arch];
  if (!pair) {
    const supported = Object.keys(defaults).map(a => `'${a}'`).join(', ');
    throw new Error(`Unsupported architecture: ${arch} on platform ${platform}. Supported architectures: ${supported}`);
  }
  return {
    platform,
    arch,
    xiaomimimo: pair[0],
    tui: pair[1],
  };
}

function executableName(base, platform) {
  return platform === "win32" ? `${base}.exe` : base;
}

function releaseBaseUrl(version, repo = "xyuai/XiaomiMiMo-TUI") {
  const override =
    process.env.XIAOMIMIMO_TUI_RELEASE_BASE_URL || process.env.XIAOMIMIMO_RELEASE_BASE_URL;
  if (override) {
    const trimmed = String(override).trim();
    return trimmed.endsWith("/") ? trimmed : `${trimmed}/`;
  }
  return `https://github.com/${repo}/releases/download/v${version}/`;
}

function releaseAssetUrl(baseName, version, repo = "xyuai/XiaomiMiMo-TUI") {
  return new URL(baseName, releaseBaseUrl(version, repo)).toString();
}

function checksumManifestUrl(version, repo = "xyuai/XiaomiMiMo-TUI") {
  return releaseAssetUrl(CHECKSUM_MANIFEST, version, repo);
}

function releaseBinaryDirectory() {
  return path.join(__dirname, "..", "bin", "downloads");
}

function allAssetNames() {
  const names = [];
  for (const platformAssets of Object.values(ASSET_MATRIX)) {
    for (const pair of Object.values(platformAssets)) {
      names.push(pair[0], pair[1]);
    }
  }
  return Array.from(new Set(names));
}

function allReleaseAssetNames() {
  return [...allAssetNames(), CHECKSUM_MANIFEST];
}

module.exports = {
  allAssetNames,
  allReleaseAssetNames,
  CHECKSUM_MANIFEST,
  checksumManifestUrl,
  detectBinaryNames,
  executableName,
  releaseAssetUrl,
  releaseBaseUrl,
  releaseBinaryDirectory,
};
