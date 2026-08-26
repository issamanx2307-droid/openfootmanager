import { readFile, stat } from "node:fs/promises";
import { resolve } from "node:path";

const projectRoot = resolve(import.meta.dirname, "..");

const assets = [
  {
    path: "public/openfootmanager_icon.png",
    type: "png",
    purpose: "Desktop and browser application icon",
  },
  { path: "public/openfootlogo.svg", type: "svg", purpose: "Primary wordmark" },
  { path: "public/openfootball.svg", type: "svg", purpose: "Header wordmark" },
  { path: "public/tauri.svg", type: "svg", purpose: "Tauri fallback branding" },
  { path: "images/openfoot.svg", type: "svg", purpose: "Source wordmark" },
  {
    path: "images/openfootball_symbol.svg",
    type: "svg",
    purpose: "Source symbol mark",
  },
  {
    path: "images/screenshots/manage_squad.png",
    type: "png",
    purpose: "Squad feature reference",
  },
  {
    path: "images/screenshots/matchlive.png",
    type: "png",
    purpose: "Live-match feature reference",
  },
  {
    path: "images/screenshots/training.png",
    type: "png",
    purpose: "Training feature reference",
  },
];

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function verifyAsset(asset) {
  const assetPath = resolve(projectRoot, asset.path);
  const metadata = await stat(assetPath);
  assert(metadata.isFile(), `${asset.path} is not a file`);
  assert(metadata.size > 0, `${asset.path} is empty`);

  const content = await readFile(assetPath);
  if (asset.type === "png") {
    assert(
      content.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
      `${asset.path} is not a valid PNG`,
    );
    return;
  }

  const svg = content.toString("utf8").trimStart();
  assert(svg.startsWith("<svg") || svg.startsWith("<?xml"), `${asset.path} is not SVG`);
  assert(/<svg\b/.test(svg), `${asset.path} has no SVG root element`);
  assert(!/<script\b/i.test(svg), `${asset.path} must not contain executable script`);
}

try {
  await Promise.all(assets.map(verifyAsset));
  console.log(`Asset audit passed: ${assets.length} tracked assets verified.`);
} catch (error) {
  console.error(`Asset audit failed: ${error.message}`);
  process.exitCode = 1;
}
