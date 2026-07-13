{
  lib,
  rustPlatform,
  fetchFromGitHub,
  runCommand,
  librsvg,
  makeWrapper,
}:

let
  flag-icons = fetchFromGitHub {
    owner = "lipis";
    repo = "flag-icons";
    rev = "v7.5.0";
    hash = "sha256-weFylGPSTgckUMujocTxUVmvNObKoa/6vgVd4cv4lOU=";
  };

  # Square (1x1) flag SVGs rasterized at the SNI pixmap sizes tailflag serves.
  flagPngs = runCommand "tailflag-flags" { nativeBuildInputs = [ librsvg ]; } ''
    for size in 24 48; do
      mkdir -p "$out/$size"
      for svg in ${flag-icons}/flags/1x1/*.svg; do
        cc=$(basename "$svg" .svg)
        rsvg-convert -w "$size" -h "$size" "$svg" -o "$out/$size/$cc.png"
      done
    done
  '';
in
rustPlatform.buildRustPackage {
  pname = "tailflag";
  version = "0.0.1";

  src = lib.cleanSourceWith {
    src = lib.cleanSource ./.;
    # keep local `cargo build` artifacts out of the store
    filter = path: _type: baseNameOf path != "target";
  };

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ makeWrapper ];

  postFixup = ''
    wrapProgram $out/bin/tailflag \
      --set-default TAILFLAG_FLAG_DIR ${flagPngs}
  '';

  meta = {
    description = "Tray icon showing the Tailscale exit-node status with a country flag";
    mainProgram = "tailflag";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
