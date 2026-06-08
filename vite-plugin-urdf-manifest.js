import fs from "node:fs";
import path from "node:path";

/**
 * @param {string} directory
 * @returns {string}
 */
function humanize(directory) {
  return directory.replace(/_/g, " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

/**
 * Generates `static/assets/manifest.json` listing every robot directory that
 * contains a `.urdf`, so the frontend can discover bundled models at runtime
 * without hardcoding paths. Regenerates on asset changes during dev.
 *
 * @returns {import('vite').Plugin}
 */
export default function urdfManifest() {
  const assetsDir = path.resolve("static/assets");
  const manifestPath = path.join(assetsDir, "manifest.json");

  function generateManifest() {
    if (!fs.existsSync(assetsDir)) return;
    const entries = [];
    for (const dir of fs.readdirSync(assetsDir, { withFileTypes: true })) {
      if (!dir.isDirectory()) continue;
      const urdf = fs
        .readdirSync(path.join(assetsDir, dir.name))
        .find((file) => file.endsWith(".urdf"));
      if (urdf) entries.push({ name: humanize(dir.name), directory: dir.name, urdf });
    }
    const next = JSON.stringify(entries, null, 2) + "\n";
    // Skip the write when unchanged: rewriting bumps mtime, which Vite reads as
    // a public-asset change and answers with a full page reload.
    if (fs.existsSync(manifestPath) && fs.readFileSync(manifestPath, "utf-8") === next) return;
    fs.writeFileSync(manifestPath, next);
  }

  return {
    name: "urdf-manifest",
    buildStart() {
      generateManifest();
    },
    configureServer(server) {
      server.watcher.add(assetsDir);
      server.watcher.on("all", (_event, filePath) => {
        if (filePath.startsWith(assetsDir) && filePath !== manifestPath) generateManifest();
      });
      generateManifest();
    },
  };
}
