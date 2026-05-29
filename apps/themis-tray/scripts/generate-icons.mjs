import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Resvg } from "@resvg/resvg-js";
import pngToIco from "png-to-ico";
import png2icons from "png2icons";

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, "..", "src-tauri", "icons");
const svg = readFileSync(join(iconsDir, "themis-icon.svg"), "utf8");

function renderPng(size) {
  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: size },
    background: "transparent",
  });
  return resvg.render().asPng();
}

const sizes = [
  ["32x32.png", 32],
  ["128x128.png", 128],
  ["256x256.png", 256],
  ["icon.png", 256],
];

for (const [name, size] of sizes) {
  writeFileSync(join(iconsDir, name), renderPng(size));
  console.log(`wrote ${name} (${size}px)`);
}

const icoSizes = [16, 24, 32, 48, 64, 128, 256].map((size) => renderPng(size));
writeFileSync(join(iconsDir, "icon.ico"), await pngToIco(icoSizes));
console.log("wrote icon.ico");

const icns = png2icons.createICNS(renderPng(512), png2icons.BICUBIC, 0, false);
if (icns) {
  writeFileSync(join(iconsDir, "icon.icns"), icns);
  console.log("wrote icon.icns");
}
