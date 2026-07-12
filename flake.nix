{
  description = "Tray icon showing the Tailscale exit-node status with a country flag";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system: rec {
        tailflag = nixpkgs.legacyPackages.${system}.callPackage ./default.nix { };
        default = tailflag;
      });

      overlays.default = final: _prev: {
        tailflag = final.callPackage ./default.nix { };
      };
    };
}
