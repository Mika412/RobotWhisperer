import fs from "node:fs";
import path from "node:path";

import type { Plugin } from "vite";

function humanize(directory: string): string {
  return directory.replace(/_/g, " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

export default function urdfManifest(): Plugin {
  const assetsDir = path.resolve("static/assets");
  const manifestPath = path.join(assetsDir, "manifest.json");

  function generateManifest(): void {
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
