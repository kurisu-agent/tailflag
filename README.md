# tailflag

System tray (StatusNotifierItem) icon showing the Tailscale exit-node
status — with the exit node's country flag when one is active.

| State | Icon |
|---|---|
| tailscale stopped / logged out | dim 3×3 dot grid |
| broken: no `tailscale` binary, daemon unreachable | dot grid with red bottom row |
| running but network unreachable (tailscaled health) | dot grid with orange bottom row |
| running, no exit node | dot grid with bright bottom row |
| exit node active, country known | that country's flag |
| exit node active, country unknown | dot grid with green bottom row |

The menu shows just the exit node's hostname; the tooltip adds
city/country and online state.

Flags are drawn as circles by default; set `TAILFLAG_STYLE` to `square`,
`rounded`, `circle`, or a corner-radius fraction (`0`–`0.5`) to change
the border style.

Works with any SNI-capable tray host (COSMIC status area applet, KDE,
waybar, GNOME appindicator extension, …).

## How it works

Event-driven: subscribes to tailscaled's IPN notification bus (LocalAPI
`watch-ipn-bus` on the unix socket — the same mechanism the official GUI
clients use) and re-reads `tailscale status --json` when a notification
arrives. Between events the process sleeps in a blocking read; there are
no periodic wake-ups. If the daemon goes away it retries the
subscription every 5 seconds. The exit node's country is resolved in
order from:

1. `Peer.Location.CountryCode` (set for Mullvad exit nodes)
2. `TAILFLAG_LOCATIONS` env map — `host=cc,host2=cc2`, matched on
   `HostName`, for self-hosted exit nodes
3. the Mullvad hostname convention (`cc-city-wg-NNN.mullvad.ts.net`)

Flag icons are rasterized at build time from
[lipis/flag-icons](https://github.com/lipis/flag-icons) (no runtime SVG
dependency); the wrapper points `TAILFLAG_FLAG_DIR` at them.

## Usage

```bash
nix run github:kurisu-agent/tailflag
```

Debugging:

```bash
tailflag --status            # print the resolved state and flag lookup
TAILFLAG_DEMO=jp tailflag    # force a state: <cc> | none | stopped | error | unknown
TAILFLAG_STYLE=rounded tailflag  # border style: square | rounded | circle | 0..0.5
```

### NixOS + home-manager

```nix
# flake input
inputs.tailflag.url = "github:kurisu-agent/tailflag";

# systemd user service
systemd.user.services.tailflag = {
  Unit = {
    Description = "Tailscale exit-node tray indicator";
    PartOf = [ "graphical-session.target" ];
    After = [ "graphical-session.target" ];
  };
  Service = {
    ExecStart = lib.getExe inputs.tailflag.packages.${pkgs.system}.default;
    Restart = "on-failure";
    RestartSec = 2;
  };
  Install.WantedBy = [ "graphical-session.target" ];
};
```

## License

MIT. Flag artwork from [lipis/flag-icons](https://github.com/lipis/flag-icons) (MIT).
