{
  rustPlatform,
  lib,
  pkg-config,
  pipewire,
}:
let
  fs = lib.fileset;
  cargoPackage = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = cargoPackage.name;
  version = cargoPackage.version;

  src = fs.toSource {
    root = ./.;
    fileset = fs.unions [
      (fs.fileFilter (file: builtins.any file.hasExt [ "rs" ]) ./src)
      ./Cargo.lock
      ./Cargo.toml
    ];
  };

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];
  buildInputs = [ pipewire ];

  cargoLock.lockFile = ./Cargo.lock;
}
