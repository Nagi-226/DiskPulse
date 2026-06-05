import { existsSync, readFileSync } from "node:fs";

const checks = [];

function check(name, condition) {
  checks.push({ name, passed: Boolean(condition) });
}

function read(path) {
  if (!existsSync(path)) return "";
  return readFileSync(path, "utf8");
}

const workflow = read(".github/workflows/ci.yml");
const cargoToml = read("src-tauri/Cargo.toml");
const linuxPlatform = read("src-tauri/src/platform/linux.rs");
const docs = read("docs/linux-ci.md");

function stepBlock(name) {
  const match = workflow.match(new RegExp(`- name: ${name}[\\s\\S]*?(?=\\n      - name:|\\n$)`));
  return match?.[0] ?? "";
}

const linuxVerify = stepBlock("Verify Linux bundles");
const macVerify = stepBlock("Verify macOS bundles");

check("CI includes ubuntu-latest matrix", /ubuntu-latest/.test(workflow));
check("Linux deps include WebKitGTK 4.1", /libwebkit2gtk-4\.1-dev/.test(workflow));
check("Linux deps include AppIndicator", /libayatana-appindicator3-dev/.test(workflow));
check("Linux deps include librsvg", /librsvg2-dev/.test(workflow));
check("Linux deps include patchelf", /patchelf/.test(workflow));
check("Linux deps include libssl-dev", /libssl-dev/.test(workflow));
check("Linux deps include pkg-config", /pkg-config/.test(workflow));
check("Linux deps include libfuse2 for AppImage validation", /libfuse2/.test(workflow));
check("Linux build validates bundle files", /Verify Linux bundles/.test(workflow));
check("Linux verify step checks .deb", /deb=\(.*\.deb/.test(linuxVerify));
check("Linux verify step checks .AppImage", /appimage=\(.*\.AppImage/.test(linuxVerify));
check("macOS verify step does not check AppImage", !/AppImage/.test(macVerify));
check("Linux upload fails if bundle files are missing", /if-no-files-found:\s*error/.test(workflow));
check("Linux upload includes deb artifact path", /bundle\/deb\/\*\.deb/.test(workflow));
check("Linux upload includes AppImage artifact path", /bundle\/appimage\/\*\.AppImage/.test(workflow));
check("Linux artifact retention is configured", /retention-days:\s*14/.test(workflow));

check("Cargo has Linux-only trash dependency", /\[target\.'cfg\(target_os = "linux"\)'\.dependencies\][\s\S]*trash\s*=/.test(cargoToml));
check("Linux cleanup uses trash crate", /trash::delete/.test(linuxPlatform));
check("Linux cleanup keeps gio fallback", /gio/.test(linuxPlatform));
check("Linux has inotify parsing tests", /parse_inotify_events_reads_multiple_records/.test(linuxPlatform));
check("Linux docs exist", existsSync("docs/linux-ci.md"));
check("Linux docs mention native ubuntu-latest", /ubuntu-latest/.test(docs));
check("Linux docs mention .deb and .AppImage", /\.deb/.test(docs) && /\.AppImage/.test(docs));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  console.log(`${item.passed ? "ok" : "not ok"} - ${item.name}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} Linux CI check(s) failed.`);
  process.exit(1);
}

console.log("\nLinux CI checks passed.");
