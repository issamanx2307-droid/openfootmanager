import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { JsonSnapshotProvider, diffSnapshots, formatDiffReport, normalizeSnapshot } from "./pipeline.mjs";

const provider = new JsonSnapshotProvider("tools/data_pipeline/fixtures/snapshot-a.json");
const rawA = await provider.load();
const rawB = JSON.parse(await readFile("tools/data_pipeline/fixtures/snapshot-b.json", "utf8"));
const snapshotA = normalizeSnapshot(rawA);
const snapshotB = normalizeSnapshot(rawB);

assert.equal(snapshotA.contentHash, normalizeSnapshot(rawA).contentHash);
const diff = diffSnapshots(snapshotA, snapshotB);
assert.deepEqual(diff, [{ playerId: "alex-porter", fromClubId: "northbridge-fc", toClubId: "riverside-town" }]);
assert.match(formatDiffReport(diff), /alex-porter: northbridge-fc -> riverside-town/);
console.log("data pipeline fixtures passed");
