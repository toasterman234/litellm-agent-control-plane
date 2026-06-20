import fs from "node:fs";
import path from "node:path";

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function fail(message) {
  console.error(`manifest validation failed: ${message}`);
  process.exit(1);
}

const baseDir = process.argv[2] ?? process.cwd();
const manifestPath = path.join(baseDir, "manifests", "repos.json");
const wavesDir = path.join(baseDir, "manifests", "waves");
const tiersPath = path.join(baseDir, "manifests", "policies", "repo-tiers.json");
const exceptionsPath = path.join(baseDir, "manifests", "policies", "exception-list.json");
const healthProfilesPath = path.join(
  baseDir,
  "manifests",
  "policies",
  "repo-health-profiles.json"
);

if (!fs.existsSync(manifestPath)) fail(`missing ${manifestPath}`);
if (!fs.existsSync(wavesDir)) fail(`missing ${wavesDir}`);
if (!fs.existsSync(tiersPath)) fail(`missing ${tiersPath}`);
if (!fs.existsSync(exceptionsPath)) fail(`missing ${exceptionsPath}`);
if (!fs.existsSync(healthProfilesPath)) fail(`missing ${healthProfilesPath}`);

const manifest = readJson(manifestPath);
const tiers = readJson(tiersPath);
const exceptions = readJson(exceptionsPath);
const healthProfiles = readJson(healthProfilesPath);

if (!Array.isArray(manifest.repos) || manifest.repos.length === 0) {
  fail("manifests/repos.json must contain a non-empty repos array");
}

if (!tiers.tiers || typeof tiers.tiers !== "object") {
  fail("repo-tiers.json must contain a tiers object");
}

const repoIds = new Set();
const repoNames = new Set();
const tierNames = new Set(Object.keys(tiers.tiers));

for (const repo of manifest.repos) {
  if (!repo.id || typeof repo.id !== "string") fail("each repo must have a string id");
  if (!repo.repo || typeof repo.repo !== "string") fail(`repo ${repo.id} must have a repo value`);
  if (!repo.default_branch || typeof repo.default_branch !== "string") {
    fail(`repo ${repo.id} must have a default_branch`);
  }
  if (typeof repo.enabled !== "boolean") fail(`repo ${repo.id} must declare enabled as boolean`);
  if (!Array.isArray(repo.labels)) fail(`repo ${repo.id} must declare labels as an array`);
  if (!Array.isArray(repo.operations) || repo.operations.length === 0) {
    fail(`repo ${repo.id} must declare at least one operation`);
  }
  if (!tierNames.has(repo.tier)) fail(`repo ${repo.id} references unknown tier ${repo.tier}`);
  if (repoIds.has(repo.id)) fail(`duplicate repo id ${repo.id}`);
  if (repoNames.has(repo.repo)) fail(`duplicate repo name ${repo.repo}`);
  repoIds.add(repo.id);
  repoNames.add(repo.repo);
}

if (!Array.isArray(exceptions.exceptions)) {
  fail("exception-list.json must contain an exceptions array");
}

if (!healthProfiles.profiles || typeof healthProfiles.profiles !== "object") {
  fail("repo-health-profiles.json must contain a profiles object");
}

if (!healthProfiles.profiles.default) {
  fail("repo-health-profiles.json must define a default profile");
}

for (const exception of exceptions.exceptions) {
  if (!repoIds.has(exception.repo_id)) {
    fail(`exception references unknown repo_id ${exception.repo_id}`);
  }
  if (!Array.isArray(exception.disabled_operations)) {
    fail(`exception ${exception.repo_id} must declare disabled_operations`);
  }
}

for (const [profileName, profile] of Object.entries(healthProfiles.profiles)) {
  if (!Array.isArray(profile.required_files)) {
    fail(`health profile ${profileName} must declare required_files`);
  }
  if (!Array.isArray(profile.notes)) {
    fail(`health profile ${profileName} must declare notes`);
  }
}

for (const profileName of Object.keys(healthProfiles.profiles)) {
  if (profileName === "default") continue;
  if (!repoIds.has(profileName)) {
    fail(`health profile ${profileName} does not match any repo id`);
  }
}

const waveFiles = fs
  .readdirSync(wavesDir)
  .filter((entry) => entry.endsWith(".json"))
  .sort();

if (waveFiles.length === 0) fail("manifests/waves must contain at least one wave");

for (const file of waveFiles) {
  const wave = readJson(path.join(wavesDir, file));
  if (!wave.name || typeof wave.name !== "string") fail(`${file} must have a string name`);
  if (!Array.isArray(wave.repo_ids)) fail(`${file} must have a repo_ids array`);
  for (const repoId of wave.repo_ids) {
    if (!repoIds.has(repoId)) fail(`${file} references unknown repo_id ${repoId}`);
  }
}

console.log(
  `validated ${manifest.repos.length} repos, ${waveFiles.length} waves, ${exceptions.exceptions.length} exceptions, and ${Object.keys(healthProfiles.profiles).length} health profiles`
);
