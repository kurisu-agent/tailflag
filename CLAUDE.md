# CLAUDE.md

## What this is

`tailflag` — a Rust SNI (StatusNotifierItem) tray icon showing Tailscale
exit-node status with a country flag. Single binary, no config file; all
knobs are env vars (see README). Distributed as a nix flake.

## Layout

- `src/main.rs` — the whole app: state machine, IPN-bus subscription,
  icon rendering (procedural dot grid + flag PNG loading), SNI tray impl.
- `default.nix` — `buildRustPackage` + build-time rasterization of
  lipis/flag-icons SVGs to PNGs (a `tailflag-flags` derivation, wired
  via `TAILFLAG_FLAG_DIR` in the wrapper).
- `flake.nix` — packages (x86_64/aarch64-linux) + overlay.
- `.github/workflows/ci.yml` — runs ONLY on `v*` tags: nix build, smoke
  test, then a GitHub release whose body is the top CHANGELOG section.

## Key design points

- **Event-driven, never poll.** Updates come from tailscaled's IPN bus
  (LocalAPI `watch-ipn-bus?mask=130` over the unix socket, HTTP/1.0 so
  the stream is unchunked). `tailscale status --json` is re-read only
  when a notification arrives. Don't add timers.
- Network-down comes from health warnings with `ImpactsConnectivity`;
  mask bit 128 delivers the initial health snapshot at subscribe time.
- Icon states are documented in the README table — keep it in sync.
- Verification without a real tailnet: `--status`, `TAILFLAG_DEMO=<cc>`,
  `TAILFLAG_DEMO_OFFLINE=1`, and D-Bus introspection of the menu
  (`busctl --user call <sni-name> /MenuBar com.canonical.dbusmenu
  GetLayout iias -- 0 -1 0`).

## Releasing

Releases are driven by version tags; commits to main do NOT trigger CI.

1. Bump the version in `Cargo.toml`, `Cargo.lock` (the `tailflag`
   package entry), and `default.nix` — keep all three in sync.
2. **Add a `## vX.Y.Z — YYYY-MM-DD` section at the TOP of
   `CHANGELOG.md`** describing the changes. The release workflow lifts
   the topmost section verbatim into the GitHub release body, so an
   unmaintained changelog means an empty/wrong release.
3. Commit, then tag and push:
   `git tag vX.Y.Z && git push && git push --tags`
4. The workflow builds, smoke-tests, and publishes the release.

## Conventions

- This repo is public — no secrets, no references to private
  infrastructure or private repos in code, docs, or commit messages.
