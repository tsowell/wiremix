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
      ./build.rs
      ./wiremix.desktop
      ./wiremix.toml
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

  # Vendor default configuration for reference or for wrapping
  # without having to commit the file to a git repository.
  postInstall = ''
    mkdir -p $out/share
    install -Dm755 ${./wiremix.toml} $out/share/wiremix.toml
    install -Dm644 ${./wiremix.desktop} $out/share/applications/wiremix.desktop
  '';
}
