import { existsSync, readFileSync } from "node:fs";

const checks = [];

function check(name, condition) {
  checks.push({ name, passed: Boolean(condition) });
}

function read(path) {
  if (!existsSync(path)) return "";
  return readFileSync(path, "utf8");
}

function versionAtLeast(actual, minimum) {
  const actualParts = actual.split(".").map(Number);
  const minimumParts = minimum.split(".").map(Number);
  for (let i = 0; i < Math.max(actualParts.length, minimumParts.length); i += 1) {
    const left = actualParts[i] ?? 0;
    const right = minimumParts[i] ?? 0;
    if (left > right) return true;
    if (left < right) return false;
  }
  return true;
}

const packageJson = JSON.parse(read("package.json"));
const cargoToml = read("src-tauri/Cargo.toml");
const tauriConf = JSON.parse(read("src-tauri/tauri.conf.json"));
const relay = read("src-tauri/src/relay/mod.rs");
const hubRegistry = read("src-tauri/src/hub/registry.rs");
const hubMod = read("src-tauri/src/hub/mod.rs");
const lib = read("src-tauri/src/lib.rs");
const hook = read("src/hooks/useRemoteDevice.ts");
const types = read("src/types.ts");
const changelog = read("CHANGELOG.md");
const progress = read("PROGRESS.md");

const cargoVersion = cargoToml.match(/^version = "([^"]+)"/m)?.[1] ?? "0.0.0";

check("package version is 0.9.2 or newer", versionAtLeast(packageJson.version, "0.9.2"));
check("Cargo version is 0.9.2 or newer", versionAtLeast(cargoVersion, "0.9.2"));
check("Tauri config version is 0.9.2 or newer", versionAtLeast(tauriConf.version, "0.9.2"));
check("relay client keeps persistent cloud bridge mode", /RelayClientHandle/.test(relay) && /cloud_bridge/.test(relay));
check("relay routes envelopes to target peers", /RelayPeer/.test(relay) && /Target device/.test(relay) && /RelayServerMessage::Routed/.test(relay));
check("relay lists registered cloud peers", /RelayClientMessage::ListDevices/.test(relay) && /RelayServerMessage::DeviceList/.test(relay));
check("relay can execute read-only hub commands and return hub_response", /hub_command/.test(relay) && /hub_response/.test(relay) && /handle_request/.test(relay));
check("relay exposes route_hub_request", /pub fn route_hub_request/.test(relay));
check("hub registry stores cloud transport", /upsert_cloud/.test(hubRegistry) && /transport: "cloud"/.test(hubRegistry));
check("hub discovery falls back to relay cloud devices", /discover_devices/.test(hubMod) && /list_cloud_devices/.test(hubMod));
check("cloud query IPC is registered", /fn query_cloud_device/.test(lib) && /query_cloud_device/.test(lib));
check("connect_relay syncs cloud devices into hub", /sync_cloud_devices\(relay::list_cloud_devices\(\)\)/.test(lib));
check("frontend DeviceInfo includes transport", /transport: "lan" \| "cloud"/.test(types));
check("frontend hook manages relay status and cloud devices", /relayStatus/.test(hook) && /connectRelay/.test(hook) && /disconnectRelay/.test(hook));
check("frontend hook routes cloud queries through IPC", /query_cloud_device/.test(hook));
check("changelog has v0.9.2 entry", /## \[0\.9\.2\]/.test(changelog));
check("progress marks current version v0.9.2", /Current version\*\*: `v0\.9\.2`/.test(progress));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  console.log(`${item.passed ? "ok" : "not ok"} - ${item.name}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} M3 cloud sync check(s) failed.`);
  process.exit(1);
}

console.log("\nM3 cloud sync checks passed.");
