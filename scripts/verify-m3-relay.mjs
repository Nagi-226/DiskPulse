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
const relay = read("src-tauri/src/relay/mod.rs");
const relayBin = read("src-tauri/src/bin/diskpulse-relay.rs");
const lib = read("src-tauri/src/lib.rs");
const types = read("src/types.ts");
const changelog = read("CHANGELOG.md");
const progress = read("PROGRESS.md");

check("package version is 0.9.1", packageJson.version === "0.9.1");
check("Cargo version is 0.9.1", /version = "0\.9\.1"/.test(cargoToml));
check("Tauri config version is 0.9.1", tauriConf.version === "0.9.1");
check("relay module exists", existsSync("src-tauri/src/relay/mod.rs"));
check("relay module defines status", /pub struct RelayStatus/.test(relay));
check("relay module defines cloud devices", /pub struct CloudDevice/.test(relay));
check("relay module defines envelope", /pub struct RelayEnvelope/.test(relay));
check("relay validates websocket URLs", /validate_relay_url/.test(relay) && /wss:\/\//.test(relay));
check("relay blocks non-read-only hub commands", /validate_relay_envelope/.test(relay) && /is_allowed_remote_command/.test(relay));
check("relay runtime starts websocket server", /pub struct RelayRuntime/.test(relay) && /TcpListener::bind/.test(relay));
check("relay server accepts register handshake", /RelayClientMessage::Register/.test(relay) && /RelayServerMessage::Registered/.test(relay));
check("relay binary exists", existsSync("src-tauri/src/bin/diskpulse-relay.rs"));
check("relay binary starts RelayRuntime", /RelayRuntime::start/.test(relayBin));
check("relay IPC commands are registered", /connect_relay/.test(lib) && /disconnect_relay/.test(lib) && /get_relay_status/.test(lib) && /list_cloud_devices/.test(lib));
check("frontend types include RelayStatus", /interface RelayStatus/.test(types));
check("frontend types include CloudDevice", /interface CloudDevice/.test(types));
check("changelog has v0.9.1 entry", /## \[0\.9\.1\]/.test(changelog));
check("progress marks current version v0.9.1", /Current version\*\*: `v0\.9\.1`/.test(progress));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  console.log(`${item.passed ? "ok" : "not ok"} - ${item.name}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} M3 relay check(s) failed.`);
  process.exit(1);
}

console.log("\nM3 relay checks passed.");
