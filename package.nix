{
  rustPlatform,
  lib,
  pkg-config,
  pipewire,
}:
let
  cargoPackage = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = cargoPackage.name;
  version = cargoPackage.version;

  src = lib.cleanSource ./.;

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];
  buildInputs = [ pipewire ];

  cargoLock.lockFile = ./Cargo.lock;
}
