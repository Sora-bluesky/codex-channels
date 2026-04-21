#!/usr/bin/env node

const fs = require("node:fs");
const https = require("node:https");
const path = require("node:path");

const packageRoot = path.resolve(__dirname, "..");
const packageJson = JSON.parse(
  fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"),
);

const version = normalizeVersion(process.env.REMOTTY_VERSION || packageJson.version);
let assetName;

try {
  assetName = assetNameFor(process.platform, process.arch);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const outputPath = path.join(packageRoot, "bin", "remotty.exe");
const downloadUrl =
  process.env.REMOTTY_BINARY_URL ||
  `https://github.com/Sora-bluesky/remotty/releases/download/${version}/${assetName}`;

if (process.env.REMOTTY_SKIP_DOWNLOAD === "1") {
  console.log("remotty: skipping binary download because REMOTTY_SKIP_DOWNLOAD=1");
  process.exit(0);
}

fs.mkdirSync(path.dirname(outputPath), { recursive: true });

download(downloadUrl, outputPath, 0)
  .then(() => {
    console.log(`remotty: installed ${assetName} from ${version}`);
  })
  .catch((error) => {
    try {
      fs.rmSync(outputPath, { force: true });
    } catch {
      // Ignore cleanup failures so the real download error is visible.
    }
    console.error(`remotty: failed to download ${assetName}`);
    console.error(error.message);
    process.exit(1);
  });

function normalizeVersion(value) {
  const trimmed = String(value).trim();
  return trimmed.startsWith("v") ? trimmed : `v${trimmed}`;
}

function assetNameFor(platform, arch) {
  if (platform !== "win32") {
    throw new Error(`remotty npm package only supports Windows; received ${platform}`);
  }

  if (arch === "x64") {
    return "remotty-x64.exe";
  }

  if (arch === "arm64") {
    return "remotty-arm64.exe";
  }

  throw new Error(`remotty npm package only supports Windows x64 and arm64; received ${arch}`);
}

function download(url, destination, redirectCount) {
  if (redirectCount > 5) {
    return Promise.reject(new Error("too many redirects while downloading remotty"));
  }

  return new Promise((resolve, reject) => {
    const request = https.get(
      url,
      {
        headers: {
          "User-Agent": `remotty-npm-installer/${packageJson.version}`,
        },
      },
      (response) => {
        const statusCode = response.statusCode || 0;
        const location = response.headers.location;

        if (statusCode >= 300 && statusCode < 400 && location) {
          response.resume();
          const nextUrl = new URL(location, url).toString();
          download(nextUrl, destination, redirectCount + 1).then(resolve, reject);
          return;
        }

        if (statusCode !== 200) {
          response.resume();
          reject(new Error(`download failed with HTTP ${statusCode}: ${url}`));
          return;
        }

        const file = fs.createWriteStream(destination, { mode: 0o755 });
        response.pipe(file);
        file.on("finish", () => {
          file.close(resolve);
        });
        file.on("error", reject);
      },
    );

    request.on("error", reject);
    request.setTimeout(30000, () => {
      request.destroy(new Error("download timed out"));
    });
  });
}
