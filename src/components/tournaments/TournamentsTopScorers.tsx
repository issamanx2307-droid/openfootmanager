import { useTranslation } from "react-i18next";

import ContextMenu from "../ContextMenu";
import {
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import { Card, CardHeader, CardBody } from "../ui";
import type { TopScorerEntry } from "./TournamentsTab.helpers";

interface TournamentsTopScorersProps {
  topScorers: TopScorerEntry[];
  onSelectTeam: (id: string) => void;
  onSelectPlayer?: (id: string) => void;
}

export default function TournamentsTopScorers({
  topScorers,
  onSelectTeam,
  onSelectPlayer,
}: TournamentsTopScorersProps) {
  const { t } = useTranslation();

  const menuItems = (playerId: string, teamId?: string | null) => {
    const items = [];

    if (typeof onSelectPlayer === "function") {
      items.push(buildViewProfileMenuItem(t, () => onSelectPlayer(playerId)));
    }

    if (teamId) {
      items.push(buildViewTeamMenuItem(t, () => onSelectTeam(teamId)));
    }

    return items;
  };

  return (
    <Card>
      <CardHeader>{t("tournaments.topScorers")}</CardHeader>
      <CardBody className="p-0">
        {topScorers.length === 0 ? (
          <p className="p-4 text-sm text-gray-400 dark:text-gray-500 text-center">
            {t("tournaments.noGoals")}
          </p>
        ) : (
          <div className="divide-y divide-gray-100 dark:divide-navy-600">
            {topScorers.map((entry, i) => {
              const player = entry.playerName;
              if (!player) return null;
              return <ContextMenu
                items={menuItems(entry.playerId, player.team_id)}
                key={entry.playerId}
              >
                <div
                  className="flex items-center px-4 py-2.5 gap-3"
                  data-testid={`tournaments-top-scorer-${entry.playerId}`}
                >
                  <span className="font-heading font-bold text-sm text-gray-400 dark:text-gray-500 w-5 text-center">
                    {i + 1}
                  </span>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
                      {player.full_name}
                    </p>
                    <p className="text-xs text-gray-400 dark:text-gray-500">
                      {player.team_name ??
                        player.team_id ??
                        ""}
                    </p>
                  </div>
                  <span className="font-heading font-bold text-lg text-accent-500 dark:text-accent-400 tabular-nums">
                    {entry.goals}
                  </span>
                </div>
              </ContextMenu>;
            })}
          </div>
        )}
      </CardBody>
    </Card>
  );
}
