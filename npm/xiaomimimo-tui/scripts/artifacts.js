const path = require("path");
const os = require("os");

const CHECKSUM_MANIFEST = "xiaomimimo-artifacts-sha256.txt";

const ASSET_MATRIX = {
  linux: {
    x64: ["xiaomimimo-linux-x64", "xiaomimimo-tui-linux-x64"],
    // arm64: ["xiaomimimo-linux-arm64", "xiaomimimo-tui-linux-arm64"], // Uncomment when binaries are available
  },
  darwin: {
    x64: ["xiaomimimo-macos-x64", "xiaomimimo-tui-macos-x64"],
    arm64: ["xiaomimimo-macos-arm64", "xiaomimimo-tui-macos-arm64"],
  },
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

function releaseBaseUrl(version, repo = "YOUR_GITHUB_USERNAME/mimotui") {
  const override =
    process.env.XIAOMIMIMO_TUI_RELEASE_BASE_URL || process.env.XIAOMIMIMO_RELEASE_BASE_URL;
  if (override) {
    const trimmed = String(override).trim();
    return trimmed.endsWith("/") ? trimmed : `${trimmed}/`;
  }
  return `https://github.com/${repo}/releases/download/v${version}/`;
}

function releaseAssetUrl(baseName, version, repo = "YOUR_GITHUB_USERNAME/mimotui") {
  return new URL(baseName, releaseBaseUrl(version, repo)).toString();
}

function checksumManifestUrl(version, repo = "YOUR_GITHUB_USERNAME/mimotui") {
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
