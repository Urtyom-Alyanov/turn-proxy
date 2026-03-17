{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustfmt" "clippy" ];
        };

        turn-proxy-pkg = pkgs.rustPlatform.buildRustPackage {
          pname = "turn-proxy-server";
          version = "1.0.6";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
        };
      in
        {
          packages.default = turn-proxy-pkg;
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustToolchain
              pkg-config
              openssl
              stdenv.cc.cc.lib
            ];
          };

          shellHook = ''
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
          '';
        }
    ) // {
      nixosModules.default = { config, lib, pkgs, ... }:
       let
         cfg = config.services.turn-proxy;
       in
       {
        options.services.turn-proxy-server = {
          enable = lib.mkEnableOption "DTLS Turn Proxy Server";
          package = lib.mkOption {
            type = lib.types.package;
            default = self.packages.${pkgs.system}.default;
            description = "Package with turn proxy";
          };
          configPath = lib.mkOption {
            type = lib.types.path;
            default = "/etc/turn-proxy/server/config.toml";
            description = "Path to config.toml if it using";
          };
          config = {
            listeningOn = lib.mkOption {
              type = lib.types.str;
              default = "0.0.0.0:56000";
              description = "Address to listening";
            };
            proxyInto = lib.mkOption {
              type = lib.types.str;
              description = "Address of UDP-based application";
            };
          };
        };

        config = lib.mkIf cfg.enable {
          systemd.services.turn-proxy-server = {
            description = "DTLS TURN Proxy Server";
            after = [ "network.target" ];
            wantedBy = [ "multi-user.target" ];
            serviceConfig = {
              ExecStart = "${cfg.package}/bin/turn-proxy-server ${if cfg.configPath then "--config=${cfg.configPath}" else "--no-config"} --listening-on=${cfg.config.listeningOn} --proxy-into=${cfg.config.proxyInto}";
              Restart = "always";
              LimitNOFILE = 65535;
            };
          };
        };
      };
    };
}