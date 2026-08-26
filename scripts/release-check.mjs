import { spawnSync } from "node:child_process";

const quick = process.argv.includes("--quick");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";

const checks = [
  [npm, ["run", "lint"]],
  [npm, ["run", "audit:assets"]],
  [npm, ["run", "build"]],
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
  const frontendShards = Array.from({ length: 4 }, (_, index) => [
    npm,
    [
      "test",
      "--",
      "--pool=threads",
      "--maxWorkers=4",
      `--shard=${index + 1}/4`,
      "--reporter=dot",
    ],
  ]);
  checks.splice(3, 0, ...frontendShards);
  checks.push([cargo, ["test", "--manifest-path", "src-tauri/Cargo.toml", "--workspace", "--quiet"]]);
}

for (const [command, args] of checks) {
  console.log(`\n> ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    shell: process.platform === "win32",
    stdio: "inherit",
  });
  if (result.error || result.status !== 0) {
    console.error(result.error?.message ?? `Release check failed with exit code ${result.status}.`);
    process.exit(result.status ?? 1);
  }
}

console.log(`\n${quick ? "Quick" : "Full"} release check passed.`);
