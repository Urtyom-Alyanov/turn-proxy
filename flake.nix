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

        cargo = fromTOML (builtins.readFile ./Cargo.toml);
        version = cargo.workspace.package.version;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustfmt" "clippy" ];
        };

        makePkg = path: pkgs.callPackage path {
                  lib = pkgs.lib;
                  inherit version;
                };
      in
        {
          packages = {
            server = makePkg ./server/nix/package.nix;
            client = makePkg ./client/nix/package.nix;
          };
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustToolchain
              pkg-config
              openssl
              stdenv.cc.cc.lib
            ];

            shellHook = ''
              export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
            '';
          };
        }
    ) // {
      nixosModules = {
        server = import ./server/nix/module.nix self;
        client = import ./client/nix/module.nix self;
        default = { ... }: {
          imports = [ self.nixosModules.server self.nixosModules.client ];
        };
      };
    };
}
