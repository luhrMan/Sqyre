const fs = require("fs");

const names = [
  "ACTIONS_RESULTS_URL",
  "ACTIONS_CACHE_URL",
  "ACTIONS_CACHE_SERVICE_V2",
  "ACTIONS_RUNTIME_TOKEN",
];

const out = process.env.GITHUB_ENV;
if (!out) process.exit(0);

const found = names.filter((n) => process.env[n]);
if (found.length) {
  fs.appendFileSync(out, found.map((n) => `${n}=${process.env[n]}\n`).join(""));
}
console.log(`gha-runtime-env: exported ${found.join(", ") || "(none)"}`);
