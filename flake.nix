{
  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.stable.latest.default;
        nativeBuildInputs = with pkgs; [
          rust
          pkg-config
          clang
        ];
        buildInputs = with pkgs; [
          pipewire
        ];
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs;
          inherit buildInputs;
          inherit LIBCLANG_PATH;
        };

        packages.default = let
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        in pkgs.rustPlatform.buildRustPackage rec {
          inherit nativeBuildInputs;
          inherit buildInputs;
          inherit LIBCLANG_PATH;

          pname = cargoToml.package.name;
          version = cargoToml.package.version;

          src = self;

          cargoLock.lockFile = ./Cargo.lock;
        };
      }
    );
}
