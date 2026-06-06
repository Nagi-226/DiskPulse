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
const packageJson = JSON.parse(read("package.json"));
const readinessDocs = read("docs/m1-release-readiness.md");

function stepBlock(name) {
  const match = workflow.match(new RegExp(`- name: ${name}[\\s\\S]*?(?=\\n      - name:|\\n$)`));
  return match?.[0] ?? "";
}

const linuxVerify = stepBlock("Verify Linux bundles");
const macVerify = stepBlock("Verify macOS bundles");
const unsignedUpload = stepBlock("Upload unsigned Windows artifacts for SignPath");
const signedVerify = stepBlock("Verify signed Windows artifacts");

check("package version is v0.8.x or newer after M1", /^(0\.(8|9)\.\d+|[1-9]\d*\.\d+\.\d+)$/.test(packageJson.version));
check("Linux verify step checks .deb", /bundle\/deb\/\*\.deb|deb=\(.*\.deb/.test(linuxVerify));
check("Linux verify step checks .AppImage", /bundle\/appimage\/\*\.AppImage|appimage=\(.*\.AppImage/.test(linuxVerify));
check("macOS verify step checks only .dmg", /bundle\/dmg\/\*\.dmg|dmg=\(.*\.dmg/.test(macVerify) && !/AppImage/.test(macVerify));
check("Unsigned Windows upload fails when bundles are missing", /if-no-files-found:\s*error/.test(unsignedUpload));
check("Signed Windows artifacts are verified after SignPath", /signed-windows/.test(signedVerify) && /Get-ChildItem|Test-Path|shopt/.test(signedVerify));
check("CI references SIGNPATH_ORGANIZATION_ID secret", /secrets\.SIGNPATH_ORGANIZATION_ID/.test(workflow));
check("M1 readiness docs exist", existsSync("docs/m1-release-readiness.md"));
check("M1 readiness docs keep SignPath external", /SignPath[\s\S]*external|external[\s\S]*SignPath/.test(readinessDocs));
check("M1 readiness docs keep Linux native runner pending", /Linux[\s\S]*native runner[\s\S]*pending|native runner[\s\S]*pending[\s\S]*Linux/.test(readinessDocs));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  console.log(`${item.passed ? "ok" : "not ok"} - ${item.name}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} M1 release readiness check(s) failed.`);
  process.exit(1);
}

console.log("\nM1 release readiness checks passed.");
