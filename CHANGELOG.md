# Changelog

## v0.0.1 — 2026-07-13

Initial release.

- SNI tray icon showing Tailscale exit-node status: country flag when an
  exit node is active (rasterized at build time from lipis/flag-icons),
  Tailscale-style dot grid otherwise — dim (stopped/logged out), bright
  bottom row (connected, no exit node), green bottom row (exit node with
  unknown country), orange bottom row (network down), red bottom row
  (tailscale broken/missing).
- Event-driven: subscribes to tailscaled's IPN bus (LocalAPI
  `watch-ipn-bus`, mask 130 for initial state + health snapshot); no
  polling, no periodic wake-ups. Reconnects every 5s while the daemon
  is unreachable.
- Offline detection from health warnings with `ImpactsConnectivity`.
- Country resolution: Mullvad `Peer.Location`, `TAILFLAG_LOCATIONS`
  env map for self-hosted nodes, Mullvad hostname convention.
- Tooltip + click menu: status headline ("Tailscale Exit Node
  Connected") and location-first detail ("Amsterdam, NL — nl-ams-wg-001").
- Configurable flag border: `TAILFLAG_STYLE` = `square` | `rounded` |
  `circle` (default) | corner-radius fraction `0`–`0.5`.
- Debug helpers: `--status`, `TAILFLAG_DEMO`, `TAILFLAG_DEMO_OFFLINE`,
  `TAILFLAG_SOCKET`, `TAILFLAG_FLAG_DIR`.
