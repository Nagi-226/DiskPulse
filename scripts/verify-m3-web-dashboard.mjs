import { existsSync, readFileSync } from "node:fs";

const checks = [];

function check(name, condition) {
  checks.push({ name, passed: Boolean(condition) });
}

function read(path) {
  if (!existsSync(path)) return "";
  return readFileSync(path, "utf8");
}

const web = read("src-tauri/src/web/mod.rs");
const lib = read("src-tauri/src/lib.rs");
const packageJson = JSON.parse(read("package.json"));
const progress = read("PROGRESS.md");
const codex = read("CODEX.md");

check("web module exists", existsSync("src-tauri/src/web/mod.rs"));
check("web runtime starts localhost server", /pub struct WebDashboardRuntime/.test(web) && /TcpListener::bind\(\("127\.0\.0\.1", port\)\)/.test(web));
check("web health endpoint exists", /\/api\/health/.test(web) && /diskpulse-web/.test(web));
check("web read-only whitelist exists", /is_web_read_only_command/.test(web) && /is_allowed_remote_command/.test(web));
check("web lifecycle API exists", /pub fn start\(port: u16\)/.test(web) && /pub fn stop\(\)/.test(web) && /pub fn get_status\(\)/.test(web));
check("web IPC commands are registered", /start_web_dashboard/.test(lib) && /stop_web_dashboard/.test(lib) && /get_web_dashboard_status/.test(lib));
check("web tests cover startup health and safety", /web_dashboard_serves_health_on_localhost/.test(web) && /web_dashboard_allows_only_read_only_commands/.test(web));
check("package exposes web dashboard verification", /verify:m3-web-dashboard/.test(JSON.stringify(packageJson.scripts ?? {})));
check("progress marks v0.9.3 local complete", /v0\.9\.3.*Web Dashboard.*Local|Web Dashboard local implementation/.test(progress));
check("CODEX marks v0.9.3 local complete", /Task AV.*Web Dashboard[\s\S]*Local Complete/.test(codex));
check("web static asset serving exists", /serve_static_asset/.test(web) && /dist-web-dashboard/.test(web));
check("web REST command bridge exists", /\/api\/cmd\//.test(web) && /handle_web_command/.test(web));
check("web blocks cleanup commands", /clean_items/.test(web) && /requires desktop confirmation/.test(web));
check("web socket route is exposed", /\/ws/.test(web) && /WebSocket/.test(web));
check("web API bridge source exists", existsSync("src/api/index.ts"));
check("web API bridge auto-detects Tauri vs browser", /__TAURI_INTERNALS__|isTauriRuntime/.test(read("src/api/index.ts")) && /fetch\(`\/api\/cmd/.test(read("src/api/index.ts")));
check("web build aliases Tauri APIs", /isWebDashboardBuild/.test(read("vite.config.ts")) && /@tauri-apps\/api\/core/.test(read("vite.config.ts")));
check("package exposes dual web build", /build:webui/.test(JSON.stringify(packageJson.scripts ?? {})));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  console.log(`${item.passed ? "ok" : "not ok"} - ${item.name}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} M3 web dashboard check(s) failed.`);
  process.exit(1);
}

console.log("\nM3 web dashboard scaffold checks passed.");
