{
  description = "Simple TUI audio mixer for PipeWire";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs = { self, nixpkgs, systems, ... }: let
    eachSystem = callback: nixpkgs.lib.genAttrs (import systems) (system: callback nixpkgs.legacyPackages.${system});
  in {
    devShells = eachSystem (pkgs: {
      default = with pkgs; mkShell {
        packages = [
          rustc
          cargo
          pkg-config
          rustPlatform.bindgenHook
              
          pipewire
        ];
      };
    });

    packages = eachSystem (pkgs: let
      package = pkgs.callPackage ./package.nix {};
    in {
      default = package;
      wiremix = package;
    });
  };
}
