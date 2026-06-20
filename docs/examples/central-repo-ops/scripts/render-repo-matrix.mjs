import fs from "node:fs";

function parseArgs(argv) {
  const result = {};
  for (let index = 2; index < argv.length; index += 1) {
    const part = argv[index];
    if (!part.startsWith("--")) continue;
    result[part.slice(2)] = argv[index + 1];
    index += 1;
  }
  return result;
}

const args = parseArgs(process.argv);
const manifestPath = args.manifest;
const wavePath = args.wave;
const operation = args.operation;

if (!manifestPath || !wavePath || !operation) {
  console.error("usage: node scripts/render-repo-matrix.mjs --manifest <path> --wave <path> --operation <name>");
  process.exit(1);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const wave = JSON.parse(fs.readFileSync(wavePath, "utf8"));
const selected = new Set(wave.repo_ids ?? []);

const include = (manifest.repos ?? [])
  .filter((repo) => repo.enabled)
  .filter((repo) => selected.has(repo.id))
  .filter((repo) => Array.isArray(repo.operations) && repo.operations.includes(operation))
  .map((repo) => ({
    id: repo.id,
    repo: repo.repo,
    tier: repo.tier,
    default_branch: repo.default_branch,
    labels: repo.labels,
    operation
  }));

process.stdout.write(JSON.stringify({ include }));
