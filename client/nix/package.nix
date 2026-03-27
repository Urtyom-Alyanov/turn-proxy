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
  pname = "turn-proxy-client";
  inherit version src;

  buildAndTestSubdir = "client";

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  cargoBuildFlags = [ "-p" "turn-proxy-client" ];

  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = [ pkgs.openssl ];
}