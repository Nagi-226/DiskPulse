import { existsSync, readFileSync } from "node:fs";
import { basename } from "node:path";

const checks = [];

function check(name, condition, detail = "") {
  checks.push({ name, passed: Boolean(condition), detail });
}

function read(path) {
  if (!existsSync(path)) return "";
  return readFileSync(path, "utf8");
}

const version = JSON.parse(read("package.json")).version;
const requiredFiles = [
  ".signpath/config.yml",
  ".signpath/policies/diskpulse/release-signing.yml",
  ".github/workflows/ci.yml",
  "packaging/homebrew/diskpulse.rb",
  "docs/signing.md",
];

for (const file of requiredFiles) {
  check(`${basename(file)} exists`, existsSync(file), file);
}

const signpathConfig = read(".signpath/config.yml");
check("SignPath config names project slug", /project_slug:\s*diskpulse/.test(signpathConfig));
check("SignPath config names artifact slug", /artifact_configuration_slug:\s*windows-installers/.test(signpathConfig));
check("SignPath config names release policy", /signing_policy_slug:\s*release-signing/.test(signpathConfig));

const policy = read(".signpath/policies/diskpulse/release-signing.yml");
check("SignPath policy requires GitHub-hosted runners", /require_github_hosted:\s*true/.test(policy));
check("SignPath policy disallows reruns", /disallow_reruns:\s*true/.test(policy));

const ci = read(".github/workflows/ci.yml");
check("CI exposes actions read permission", /actions:\s*read/.test(ci));
check("CI uploads unsigned Windows artifact", /upload-unsigned-windows-artifacts/.test(ci));
check("CI fails unsigned upload if installers are missing", /upload-unsigned-windows-artifacts[\s\S]*if-no-files-found:\s*error/.test(ci));
check("CI submits SignPath request", /signpath\/github-action-submit-signing-request@v2/.test(ci));
check("CI references SIGNPATH_API_TOKEN secret", /secrets\.SIGNPATH_API_TOKEN/.test(ci));
check("CI references SIGNPATH_ORGANIZATION_ID secret", /secrets\.SIGNPATH_ORGANIZATION_ID/.test(ci));
check("CI verifies signed Windows artifacts", /Verify signed Windows artifacts/.test(ci));
check("CI uploads signed Windows artifact", /diskpulse-windows-signed/.test(ci));
check("CI fails signed upload if artifacts are missing", /diskpulse-windows-signed[\s\S]*if-no-files-found:\s*error/.test(ci));

const cask = read("packaging/homebrew/diskpulse.rb");
check("Cask uses current version", new RegExp(`version "${version}"`).test(cask));
const escapedVersion = version.replaceAll(".", "\\.");
check(
  "Cask targets the current dmg URL",
  new RegExp(
    `url "https:\\/\\/github\\.com\\/Nagi-226\\/DiskPulse\\/releases\\/download\\/v${escapedVersion}\\/DiskPulse_${escapedVersion}_x64\\.dmg"`,
  ).test(cask),
);
check("Cask preserves unsigned fallback caveat", /unsigned build/.test(cask));

const docs = read("docs/signing.md");
check("Signing docs mention SignPath Foundation", /SignPath Foundation/.test(docs));
check("Signing docs list required GitHub secrets", /SIGNPATH_API_TOKEN/.test(docs));
check("Signing docs mention Homebrew Cask", /Homebrew Cask/.test(docs));

const failed = checks.filter((item) => !item.passed);
for (const item of checks) {
  const marker = item.passed ? "ok" : "not ok";
  console.log(`${marker} - ${item.name}${item.detail ? ` (${item.detail})` : ""}`);
}

if (failed.length > 0) {
  console.error(`\n${failed.length} signing configuration check(s) failed.`);
  process.exit(1);
}

console.log("\nSigning configuration checks passed.");
