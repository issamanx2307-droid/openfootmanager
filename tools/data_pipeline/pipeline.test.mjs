import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { CsvSnapshotProvider, JsonSnapshotProvider, RATING_MODEL_VERSION, SnapshotValidationError, contentHash, createCareerSeed, createSnapshotDiff, diffSnapshots, formatDiffReport, normalizeSnapshot, ratePlayer, verifySnapshot } from "./pipeline.mjs";

const provider = new JsonSnapshotProvider("tools/data_pipeline/fixtures/snapshot-a.json");
const rawA = await provider.load();
const rawB = JSON.parse(await readFile("tools/data_pipeline/fixtures/snapshot-b.json", "utf8"));
const snapshotA = normalizeSnapshot(rawA);
const snapshotB = normalizeSnapshot(rawB);

assert.equal(snapshotA.contentHash, normalizeSnapshot(rawA).contentHash);
assert.equal(verifySnapshot(snapshotA), true);
assert.equal(snapshotA.ratingModelVersion, RATING_MODEL_VERSION);
assert.deepEqual(ratePlayer({ id: "alex-porter" }), ratePlayer({ id: "alex-porter" }));
assert.notEqual(ratePlayer({ id: "alex-porter" }).overall, ratePlayer({ id: "sam-reed" }).overall);
assert.equal(snapshotA.players.find((player) => player.id === "alex-porter").rating.overall, snapshotB.players.find((player) => player.id === "alex-porter").rating.overall);
assert.equal(contentHash({ b: { z: 1, a: 2 }, a: 1 }), contentHash({ a: 1, b: { a: 2, z: 1 } }));
const diff = diffSnapshots(snapshotA, snapshotB);
assert.deepEqual(diff, [{ playerId: "alex-porter", fromClubId: "northbridge-fc", toClubId: "riverside-town" }]);
assert.deepEqual(createSnapshotDiff(snapshotA, snapshotB).rosterMoves, diff);
assert.match(formatDiffReport(diff), /alex-porter: northbridge-fc -> riverside-town/);
assert.equal(createCareerSeed(snapshotB).sourceSnapshotHash, snapshotB.contentHash);
assert.throws(() => normalizeSnapshot({ season: "2026/27", clubs: [{ id: "club", name: "Club" }], players: [{ id: "player", name: "Player", clubId: "missing", position: "ST" }] }), SnapshotValidationError);
assert.throws(() => verifySnapshot({ ...snapshotA, contentHash: "sha256:invalid" }), SnapshotValidationError);
const csv = await new CsvSnapshotProvider("tools/data_pipeline/fixtures/snapshot-a.csv").load();
assert.equal(normalizeSnapshot(csv).players[0].id, "alex-porter");
console.log("data pipeline fixtures passed");
