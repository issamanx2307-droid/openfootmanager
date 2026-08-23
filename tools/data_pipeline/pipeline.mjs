import { createHash } from "node:crypto";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";

export const RATING_MODEL_VERSION = "albion-rating-v1";

export class SnapshotValidationError extends Error {
  constructor(errors) {
    super(`Snapshot validation failed:\n- ${errors.join("\n- ")}`);
    this.name = "SnapshotValidationError";
    this.errors = errors;
  }
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export function contentHash(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function deterministicNumber(seed, limit) {
  return createHash("sha256").update(seed).digest().readUInt32BE(0) % limit;
}

/// Generates a clearly-labelled fallback estimate when a provider has no
/// performance metrics. The stable player id keeps the uncertainty reproducible
/// across a transfer to a later snapshot.
export function ratePlayer(player) {
  const seed = `${RATING_MODEL_VERSION}:${player.id}`;
  const overall = 48 + deterministicNumber(seed, 22);
  return {
    modelVersion: RATING_MODEL_VERSION,
    origin: "albion_rating_v1",
    overall,
    potential: Math.min(99, overall + 3 + deterministicNumber(`${seed}:potential`, 13)),
    confidence: "low",
    factors: ["stable-id deterministic fallback"],
  };
}

export class JsonSnapshotProvider {
  constructor(path, provider = "manual-json") {
    this.path = path;
    this.provider = provider;
  }

  async load() {
    const raw = await readFile(resolve(this.path), "utf8");
    const input = JSON.parse(raw);
    return {
      ...input,
      provenance: { ...(input.provenance ?? {}), provider: this.provider, rawInputHash: contentHash(raw) },
    };
  }
}

function parseCsv(text) {
  const rows = [];
  let row = [];
  let value = "";
  let quoted = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (character === '"') {
      if (quoted && text[index + 1] === '"') { value += '"'; index += 1; } else quoted = !quoted;
    } else if (character === "," && !quoted) { row.push(value); value = ""; }
    else if ((character === "\n" || character === "\r") && !quoted) {
      if (character === "\r" && text[index + 1] === "\n") index += 1;
      row.push(value); value = "";
      if (row.some((cell) => cell !== "")) rows.push(row);
      row = [];
    } else value += character;
  }
  row.push(value);
  if (row.some((cell) => cell !== "")) rows.push(row);
  return rows;
}

export class CsvSnapshotProvider {
  constructor(path, provider = "manual-csv") {
    this.path = path;
    this.provider = provider;
  }

  async load() {
    const raw = await readFile(resolve(this.path), "utf8");
    const [headers, ...rows] = parseCsv(raw);
    const records = rows.map((row) => Object.fromEntries(headers.map((header, index) => [header, row[index] ?? ""])));
    const season = records.find((record) => record.season)?.season;
    return {
      season,
      clubs: records.filter((record) => record.type === "club").map(({ id, name, country }) => ({ id, name, country })),
      players: records.filter((record) => record.type === "player").map(({ id, name, clubId, position }) => ({ id, name, clubId, position })),
      provenance: { provider: this.provider, rawInputHash: contentHash(raw) },
    };
  }
}

const stringField = (value) => typeof value === "string" && value.trim().length > 0;

export function validateSnapshot(input) {
  const errors = [];
  const warnings = [];
  if (!stringField(input.season)) errors.push("season is required");
  const clubIds = new Set();
  for (const club of input.clubs ?? []) {
    if (!stringField(club.id) || !stringField(club.name)) errors.push("each club requires a non-empty id and name");
    if (clubIds.has(club.id)) errors.push(`duplicate club id: ${club.id}`);
    clubIds.add(club.id);
  }
  const playerIds = new Set();
  for (const player of input.players ?? []) {
    if (![player.id, player.name, player.clubId, player.position].every(stringField)) errors.push("each player requires id, name, clubId, and position");
    if (playerIds.has(player.id)) errors.push(`duplicate player id: ${player.id}`);
    playerIds.add(player.id);
    if (!clubIds.has(player.clubId)) errors.push(`player ${player.id} references unknown club: ${player.clubId}`);
  }
  if ((input.clubs ?? []).length === 0) errors.push("at least one club is required");
  if ((input.players ?? []).length === 0) warnings.push("snapshot has no players");
  return { errors, warnings };
}

function normalizedProvenance(provenance = {}) {
  return {
    provider: provenance.provider ?? "deterministic-fixture",
    rawInputHash: provenance.rawInputHash ?? null,
    pipelineVersion: provenance.pipelineVersion ?? "albion-data-v1",
  };
}

export function normalizeSnapshot(input) {
  const { errors } = validateSnapshot(input);
  if (errors.length) throw new SnapshotValidationError(errors);
  const clubs = [...input.clubs]
    .map(({ id, name, country = "ENG" }) => ({ id, name, country }))
    .sort((a, b) => a.id.localeCompare(b.id));
  const players = [...input.players]
    .map(({ id, name, clubId, position }) => ({ id, name, clubId, position, rating: ratePlayer({ id }) }))
    .sort((a, b) => a.id.localeCompare(b.id));
  const snapshot = {
    schemaVersion: 1,
    season: input.season,
    ratingModelVersion: RATING_MODEL_VERSION,
    clubs,
    players,
    provenance: normalizedProvenance(input.provenance),
  };
  return { ...snapshot, contentHash: contentHash(snapshot) };
}

export function verifySnapshot(snapshot) {
  const { contentHash: expected, ...content } = snapshot;
  const actual = contentHash(content);
  if (expected !== actual) throw new SnapshotValidationError(["manifest hash mismatch"]);
  return true;
}

export function diffSnapshots(before, after) {
  const prior = new Map(before.players.map((player) => [player.id, player]));
  return after.players.flatMap((player) => {
    const old = prior.get(player.id);
    return old && old.clubId !== player.clubId
      ? [{ playerId: player.id, fromClubId: old.clubId, toClubId: player.clubId }]
      : [];
  });
}

export function createSnapshotDiff(before, after) {
  const beforePlayers = new Map(before.players.map((player) => [player.id, player]));
  const afterPlayers = new Map(after.players.map((player) => [player.id, player]));
  return {
    playersAdded: [...afterPlayers.keys()].filter((id) => !beforePlayers.has(id)),
    playersRemoved: [...beforePlayers.keys()].filter((id) => !afterPlayers.has(id)),
    rosterMoves: diffSnapshots(before, after),
    identityConflicts: [],
    warnings: validateSnapshot(after).warnings,
  };
}

export function createCareerSeed(snapshot) {
  verifySnapshot(snapshot);
  return { sourceSnapshotHash: snapshot.contentHash, season: snapshot.season, clubs: snapshot.clubs, players: snapshot.players };
}

export function formatDiffReport(diff) {
  const moves = Array.isArray(diff) ? diff : diff.rosterMoves;
  if (moves.length === 0) return "# Snapshot diff\n\nNo player transfers.";
  return ["# Snapshot diff", "", "## Roster movement", ...moves.map((move) => `- ${move.playerId}: ${move.fromClubId} -> ${move.toClubId}`)].join("\n");
}

if (import.meta.url === `file://${process.argv[1]?.replaceAll("\\", "/")}`) {
  const [command, inputPath, outputPath, previousPath] = process.argv.slice(2);
  if (command !== "publish" || !inputPath || !outputPath) throw new Error("Usage: publish input.json output.json [previous.json]");
  const snapshot = normalizeSnapshot(JSON.parse(await readFile(resolve(inputPath), "utf8")));
  await mkdir(dirname(resolve(outputPath)), { recursive: true });
  await writeFile(resolve(outputPath), `${JSON.stringify(snapshot, null, 2)}\n`);
  if (previousPath) {
    const previous = JSON.parse(await readFile(resolve(previousPath), "utf8"));
    verifySnapshot(previous);
    const diff = createSnapshotDiff(previous, snapshot);
    console.log(JSON.stringify(diff, null, 2));
    console.log(formatDiffReport(diff));
  }
}
