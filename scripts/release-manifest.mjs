// Builds the updater manifest (latest.json) and SHA256SUMS for a release.
// Usage: node scripts/release-manifest.mjs <bundle-dir> <version> <owner/repo> [notes]
// The installer must already be signed by `tauri build` (a sibling .sig file must exist).
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [bundleDir, version, repo, notes = ""] = process.argv.slice(2);
if (!bundleDir || !version || !repo) {
  console.error("usage: release-manifest.mjs <bundle-dir> <version> <owner/repo> [notes]");
  process.exit(2);
}
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`not a SemVer version: ${version}`);
  process.exit(2);
}
if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) {
  console.error(`not an owner/repo: ${repo}`);
  process.exit(2);
}

const installers = readdirSync(bundleDir).filter((f) => f.endsWith("-setup.exe"));
if (installers.length !== 1) {
  console.error(`expected exactly one *-setup.exe in ${bundleDir}, found ${installers.length}`);
  process.exit(1);
}
const installer = installers[0];
if (!installer.includes(`_${version}_`)) {
  console.error(`installer ${installer} does not match version ${version}`);
  process.exit(1);
}

const signature = readFileSync(join(bundleDir, `${installer}.sig`), "utf8").trim();
if (signature.length === 0) {
  console.error("empty updater signature");
  process.exit(1);
}

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(installer)}`,
    },
  },
};
writeFileSync(join(bundleDir, "latest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

const sums = [installer, `${installer}.sig`, "latest.json"]
  .map((name) => {
    const hash = createHash("sha256").update(readFileSync(join(bundleDir, name))).digest("hex");
    return `${hash}  ${name}`;
  })
  .join("\n");
writeFileSync(join(bundleDir, "SHA256SUMS"), `${sums}\n`);

console.log(`latest.json and SHA256SUMS written for ${installer}`);
