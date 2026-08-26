# Asset manifest and QA

The tracked assets below are part of the user-facing single-player experience.
`npm run audit:assets` checks that every listed file exists, is non-empty, has
the expected PNG/SVG signature, and that SVG files contain no executable
script. It is a fast guard against broken image references and malformed asset
replacements; visual appearance is still reviewed through the acceptance
runbook.

| Asset | Purpose |
|---|---|
| `public/openfootmanager_icon.png` | Desktop and browser application icon |
| `public/openfootlogo.svg` | Primary wordmark |
| `public/openfootball.svg` | Header wordmark |
| `public/tauri.svg` | Tauri fallback branding |
| `images/openfoot.svg` | Source wordmark |
| `images/openfootball_symbol.svg` | Source symbol mark |
| `images/screenshots/manage_squad.png` | Squad feature reference |
| `images/screenshots/matchlive.png` | Live-match feature reference |
| `images/screenshots/training.png` | Training feature reference |

When adding a user-facing logo, icon, screenshot or generated image, add it to
both this table and `scripts/audit-assets.mjs`. For assets with transparency,
inspect the result against a contrasting background before accepting it, as a
raw transparent image can hide edge or alpha artifacts.
