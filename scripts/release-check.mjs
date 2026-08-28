import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";

const quick = process.argv.includes("--quick");
const isWindows = process.platform === "win32";
const npm = isWindows ? process.execPath : "npm";
const npmArgsPrefix = isWindows
  ? [
      process.env.npm_execpath ??
        join(dirname(process.execPath), "node_modules", "npm", "bin", "npm-cli.js"),
    ]
  : [];
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const npmCheck = (args) => [npm, [...npmArgsPrefix, ...args]];

const checks = [
  npmCheck(["run", "lint"]),
  npmCheck(["run", "audit:assets"]),
  npmCheck(["run", "build"]),
  [cargo, ["fmt", "--manifest-path", "src-tauri/Cargo.toml", "--all", "--", "--check"]],
  [
    cargo,
    [
      "clippy",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--workspace",
      "--all-targets",
      "--",
      "-D",
      "warnings",
    ],
  ],
  [
    cargo,
    [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "-p",
      "ofm_core",
      "--test",
      "scenario_tests",
      "--quiet",
    ],
  ],
];

if (!quick) {
  const frontendShards = Array.from({ length: 4 }, (_, index) => npmCheck([
      "test",
      "--",
      "--pool=threads",
      "--maxWorkers=4",
      `--shard=${index + 1}/4`,
      "--reporter=dot",
    ]));
  checks.splice(3, 0, ...frontendShards);
  checks.push([cargo, ["test", "--manifest-path", "src-tauri/Cargo.toml", "--workspace", "--quiet"]]);
}

for (const [command, args] of checks) {
  console.log(`\n> ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    stdio: "inherit",
  });
  if (result.error || result.status !== 0) {
    console.error(result.error?.message ?? `Release check failed with exit code ${result.status}.`);
    process.exit(result.status ?? 1);
  }
}

console.log(`\n${quick ? "Quick" : "Full"} release check passed.`);
