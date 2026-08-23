import { createHash } from "node:crypto";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const canonical = (value) => JSON.stringify(value, Object.keys(value).sort());
const hash = (value) => createHash("sha256").update(canonical(value)).digest("hex");

export function normalizeSnapshot(input) {
  const clubs = [...(input.clubs ?? [])]
    .map(({ id, name, country = "ENG" }) => ({ id, name, country }))
    .sort((a, b) => a.id.localeCompare(b.id));
  const players = [...(input.players ?? [])]
    .map(({ id, name, clubId, position }) => ({ id, name, clubId, position }))
    .sort((a, b) => a.id.localeCompare(b.id));
  const snapshot = { schemaVersion: 1, season: input.season, clubs, players };
  return { ...snapshot, contentHash: hash(snapshot) };
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

if (import.meta.url === `file://${process.argv[1]?.replaceAll("\\", "/")}`) {
  const [command, inputPath, outputPath, previousPath] = process.argv.slice(2);
  if (command !== "publish" || !inputPath || !outputPath) throw new Error("Usage: publish input.json output.json [previous.json]");
  const snapshot = normalizeSnapshot(JSON.parse(await readFile(resolve(inputPath), "utf8")));
  await mkdir(dirname(resolve(outputPath)), { recursive: true });
  await writeFile(resolve(outputPath), `${JSON.stringify(snapshot, null, 2)}\n`);
  if (previousPath) console.log(JSON.stringify(diffSnapshots(JSON.parse(await readFile(resolve(previousPath), "utf8")), snapshot), null, 2));
}
