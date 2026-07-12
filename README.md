# tailflag

System tray (StatusNotifierItem) icon showing the Tailscale exit-node
status — with the exit node's country flag when one is active.

| State | Icon |
|---|---|
| tailscale stopped / logged out / unreachable | dim 3×3 dot grid |
| running, no exit node | dot grid with bright bottom row |
| exit node active, country known | that country's flag |
| exit node active, country unknown | dot grid with green bottom row |

The tooltip and menu show the exit node's hostname, city/country, IP and
online state. The menu also offers **Disable exit node** (needs Tailscale
[operator](https://tailscale.com/kb/1080/cli#using-tailscale-without-sudo)
permission) and **Refresh**.

Works with any SNI-capable tray host (COSMIC status area applet, KDE,
waybar, GNOME appindicator extension, …).

## How it works

Polls `tailscale status --json` every 3 seconds. The exit node's country
is resolved in order from:

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
TAILFLAG_DEMO=jp tailflag    # force a state: <cc> | none | stopped | unknown
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
