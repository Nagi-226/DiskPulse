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

check("package version is 0.8.5", packageJson.version === "0.8.5");
check("Cargo version is 0.8.5", /version = "0\.8\.5"/.test(cargoToml));
check("Tauri config version is 0.8.5", tauriConf.version === "0.8.5");
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

check("Changelog has v0.8.5 entry", /## \[0\.8\.5\]/.test(changelog));
check("Progress marks current version v0.8.5", /Current version\*\*: `v0\.8\.5`/.test(progress));
check("Progress records 138 tests", /138\/138/.test(progress));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  console.log(`${item.passed ? "ok" : "not ok"} - ${item.name}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} M2 intelligence check(s) failed.`);
  process.exit(1);
}

console.log("\nM2 intelligence checks passed.");
