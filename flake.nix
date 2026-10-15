{
  description = "Simple TUI audio mixer for PipeWire";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
      ...
    }:
    let
      rustToolchain = (nixpkgs.lib.importTOML ./rust-toolchain.toml).toolchain;
      eachSystem =
        callback:
        nixpkgs.lib.genAttrs (import systems) (
          system: callback nixpkgs.legacyPackages.${system}
        );
    in
    {
      devShells = eachSystem (pkgs: {
        default =
          assert nixpkgs.lib.assertMsg (
            pkgs.rustc.version == rustToolchain.channel
          ) "rust-toolchain.toml version does not match flake";
          with pkgs;
          mkShell {
            packages = [
              rustc
              cargo
              rustfmt
              nixfmt
              clippy
              pkg-config
              rustPlatform.bindgenHook
              typos
              cargo-msrv

              pipewire
            ];
          };
      });

      packages = eachSystem (
        pkgs:
        let
          package = pkgs.callPackage ./package.nix { };
        in
        {
          default = package;
          wiremix = package;
        }
      );
    };
}
