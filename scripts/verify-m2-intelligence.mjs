import { existsSync, readFileSync } from "node:fs";

const checks = [];

function check(name, condition) {
  checks.push({ name, passed: Boolean(condition) });
}

function read(path) {
  if (!existsSync(path)) return "";
  return readFileSync(path, "utf8");
}

const packageJson = JSON.parse(read("package.json"));
const cargoToml = read("src-tauri/Cargo.toml");
const tauriConf = JSON.parse(read("src-tauri/tauri.conf.json"));
const changelog = read("CHANGELOG.md");
const progress = read("PROGRESS.md");
const fileclassMod = read("src-tauri/src/fileclass/mod.rs");
const fileclassFeatures = read("src-tauri/src/fileclass/features.rs");
const fileclassModel = read("src-tauri/src/fileclass/model.rs");
const anomalyMod = read("src-tauri/src/anomaly/mod.rs");
const anomalyAe = read("src-tauri/src/anomaly/ae.rs");
const anomalyFeatures = read("src-tauri/src/anomaly/features.rs");
const risk = read("src-tauri/src/risk/mod.rs");
const storage = read("src-tauri/src/storage/mod.rs");
const types = read("src/types.ts");
const lib = read("src-tauri/src/lib.rs");
const i18n = read("src/i18n/index.tsx");
const settingsPage = read("src/pages/Settings/index.tsx");
const modelManager = read("src-tauri/src/model_manager.rs");

function versionAtLeast(version, min) {
  const left = version.split(".").map((part) => Number.parseInt(part, 10));
  const right = min.split(".").map((part) => Number.parseInt(part, 10));
  for (let i = 0; i < Math.max(left.length, right.length); i += 1) {
    const a = left[i] ?? 0;
    const b = right[i] ?? 0;
    if (a !== b) return a > b;
  }
  return true;
}

const cargoVersion = cargoToml.match(/version = "([^"]+)"/)?.[1] ?? "0.0.0";

check("package version is 0.9.0 or newer after M2", versionAtLeast(packageJson.version, "0.9.0"));
check("Cargo version is 0.9.0 or newer after M2", versionAtLeast(cargoVersion, "0.9.0"));
check("Tauri config version is 0.9.0 or newer after M2", versionAtLeast(tauriConf.version, "0.9.0"));
check("ml-engine feature gate exists", /ml-engine = \[\]/.test(cargoToml));

check("AE module is registered", /pub mod ae;/.test(anomalyMod));
check("AE features module is registered", /pub mod features;/.test(anomalyMod));
check("AE synthetic module is registered", /pub mod synthetic;/.test(anomalyMod));
check("AE model uses 6x4x6 version marker", /ae-6x4x6-v0\.8\.4/.test(anomalyAe));
check("AE extracts six snapshot features", /as_array\(&self\) -> \[f64; 6\]/.test(anomalyFeatures));

check("File classifier Stage 3 modules are registered", /pub mod features;/.test(fileclassMod) && /pub mod model;/.test(fileclassMod));
check("File classifier extracts eight features", /as_array\(&self\) -> \[f64; 8\]/.test(fileclassFeatures));
check("File classifier exposes 12-class model", /labels: \[FileCategory; 12\]/.test(fileclassModel));
check("File classifier version marker is v0.8.5", /stage3-softmax-v0\.8\.5/.test(fileclassModel));
check("File classifier synthetic sample target is 5000+", /SYNTHETIC_TRAINING_SAMPLES: usize = 6_000/.test(fileclassModel));

check("Risk rules support file_category condition", /pub file_category: Option<String>/.test(risk));
check("Risk rules include dev_cache category rule", /file-category-dev-cache/.test(risk));
check("Risk rules include build category rule", /file-category-build/.test(risk));
check("Risk rules include dependency category rule", /file-category-dependency/.test(risk));

check("Storage module defines external storage info", /struct ExternalStorageInfo/.test(storage));
check("Storage module defines attach/detach events", /STORAGE_ATTACHED_EVENT/.test(storage) && /STORAGE_DETACHED_EVENT/.test(storage));
check("Storage module defines provider trait", /trait ExternalStorageProvider/.test(storage));
check("Storage module includes Windows WM_DEVICECHANGE model", /WM_DEVICECHANGE/.test(storage) && /DBT_DEVICEARRIVAL/.test(storage));
check("Storage module includes Linux fallback provider", /linux_mount_poll_fallback/.test(storage));
check("Storage module includes macOS fallback provider", /macos_volumes_poll_fallback/.test(storage));
check("Storage IPC commands are registered", /list_external_storage/.test(lib) && /get_storage_info/.test(lib) && /start_storage_monitor/.test(lib));
check("Frontend types include ExternalStorageInfo", /interface ExternalStorageInfo/.test(types));

check("Korean locale is registered", /locales\/ko\.json/.test(i18n) && /id: "ko"/.test(i18n));
check("Spanish locale is registered", /locales\/es\.json/.test(i18n) && /id: "es"/.test(i18n));
check("Korean locale file exists", existsSync("src/i18n/locales/ko.json"));
check("Spanish locale file exists", existsSync("src/i18n/locales/es.json"));

check("Model manager exposes 60 snapshot gate", /MIN_FINE_TUNE_SNAPSHOTS: usize = 60/.test(modelManager));
check("Model IPC commands are registered", /get_model_status/.test(lib) && /fine_tune_models/.test(lib) && /reset_models/.test(lib));
check("Frontend types include ModelStatus", /interface ModelStatus/.test(types));
check("Settings page includes AI Model tab", /AI Model/.test(settingsPage) && /ModelSettingsTab/.test(settingsPage));

check("Changelog has v0.8.5 entry", /## \[0\.8\.5\]/.test(changelog));
check("Changelog has v0.8.6 entry", /## \[0\.8\.6\]/.test(changelog));
check("Changelog has v0.9.0 entry", /## \[0\.9\.0\]/.test(changelog));
check("Progress keeps M2 v0.9.0 completion", /M2 completion\*\*: v0\.9\.0 local complete/.test(progress) || /M2 \(v0\.9\.0\)/.test(progress));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  console.log(`${item.passed ? "ok" : "not ok"} - ${item.name}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} M2 intelligence check(s) failed.`);
  process.exit(1);
}

console.log("\nM2 intelligence checks passed.");
