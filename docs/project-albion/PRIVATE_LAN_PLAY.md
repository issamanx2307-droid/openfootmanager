# Private LAN / Tailscale play

Project Albion is a private two-manager career. It does not provide public
matchmaking or an account service.

## Host a career

1. Open a saved career and select a club.
2. From the dashboard choose **Play Together**.
3. Enter a private join code and choose **Host game**.
4. The lobby fills a URL such as `http://127.0.0.1:38421`. Replace
   `127.0.0.1` with the host computer's LAN address (for example
   `192.168.1.10`) or Tailscale address before sharing it.
5. Share the resulting URL and join code with one guest. The host keeps the
   desktop app open; closing the lobby stops the bundled server.

The bundled host listens on its assigned port on all interfaces. If Windows
Firewall asks, allow the app on the intended private network only.

## Join a career

1. Open a career whose manager controls the club to claim.
2. Choose **Play Together** from the dashboard.
3. Enter the host URL and join code, then choose **Join game**.
4. The server validates the game/ruleset versions and rejects an already
   claimed club. Once connected, the lobby receives the manager-specific
   dashboard snapshot.

Both managers use **Ready to continue**. The server advances only after both
are ready and stops at a controlled club's fixture. Live-match score/time and
tactical commands are server-owned.

## Reconnect

The lobby stores the issued reconnect token locally. Choose **Reconnect** to
restore the same manager slot after a temporary connection loss. Do not use
New Game to reconnect: it creates a different local career and cannot claim a
club in the hosted one.

## Troubleshooting

- Verify both computers are on the same LAN or Tailscale tailnet.
- Confirm the port in the URL is unchanged.
- Use the exact same app/ruleset build; incompatible versions are rejected.
- If the host restarts, use **Reconnect** after it is available again.
- During a human-controlled live match, a disconnected manager pauses that
  match; reconnect to resume.
