import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  getScoutAvailability,
  type PlayerProfileScoutStatus,
  type ScoutAvailability,
} from "./PlayerProfile.scouting";

interface UseScoutPlayerFlowArgs {
  player: PlayerData;
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
}

interface UseScoutPlayerFlowResult {
  scoutAvailability: ScoutAvailability;
  scoutStatus: PlayerProfileScoutStatus;
  scoutError: string | null;
  sendScout: () => void;
}

interface ScoutPlayerState {
  status: PlayerProfileScoutStatus;
  error: string | null;
}

const INITIAL_SCOUT_PLAYER_STATE: ScoutPlayerState = {
  status: "idle",
  error: null,
};

/**
 * Sending a scout to watch this player.
 *
 * `scoutAvailability` is derived from the current status as well as the squad,
 * so the two travel together — a scout already on their way is not available
 * to send again.
 */
export function useScoutPlayerFlow({
  player,
  gameState,
  onGameUpdate,
}: UseScoutPlayerFlowArgs): UseScoutPlayerFlowResult {
  const { t } = useTranslation();
  const [scoutPlayerStates, setScoutPlayerStates] = useState<
    Record<string, ScoutPlayerState>
  >({});
  const scoutPlayerState =
    scoutPlayerStates[player.id] ?? INITIAL_SCOUT_PLAYER_STATE;
  const scoutStatus = scoutPlayerState.status;
  const scoutError = scoutPlayerState.error;

  function updateScoutPlayerState(update: Partial<ScoutPlayerState>): void {
    setScoutPlayerStates((states) => ({
      ...states,
      [player.id]: {
        ...(states[player.id] ?? INITIAL_SCOUT_PLAYER_STATE),
        ...update,
      },
    }));
  }

  const scoutAvailability = getScoutAvailability({
    staff: gameState.staff,
    scoutingAssignments: gameState.scouting_assignments || [],
    youthScoutingAssignments: gameState.youth_scouting_assignments || [],
    managerTeamId: gameState.manager.team_id,
    playerId: player.id,
    scoutStatus,
  });

  function sendScout(): void {
    const availableScout = scoutAvailability.availableScout;
    if (!availableScout || !onGameUpdate) {
      return;
    }

    void (async () => {
      updateScoutPlayerState({ status: "sending", error: null });

      try {
        const updated = await invoke<GameStateData>("send_scout", {
          scoutId: availableScout.id,
          playerId: player.id,
        });
        onGameUpdate(updated);
        updateScoutPlayerState({ status: "sent" });
      } catch (err) {
        updateScoutPlayerState({
          error: resolveTranslatedErrorMessage(err, t),
          status: "error",
        });
      }
    })();
  }

  return { scoutAvailability, scoutStatus, scoutError, sendScout };
}
