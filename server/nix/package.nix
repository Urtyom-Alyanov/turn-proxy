{ version, lib }:
let
  src = lib.cleanSourceWith {
    src = ../..;
    filter = path: type:
      let
        baseName = baseNameOf path;
      in
      (type == "directory") ||
      (baseName == "Cargo.toml" || baseName == "Cargo.lock") ||
      (lib.hasSuffix ".rs" baseName) ||
      (baseName == "rustfmt.toml");
  };
in
pkgs.rustPlatform.buildRustPackage {
  pname = "turn-proxy-server";
  inherit version src;

  buildAndTestSubdir = "server";

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  cargoBuildFlags = [ "-p" "turn-proxy-server" ];

  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = [ pkgs.openssl ];
}