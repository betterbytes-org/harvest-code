{ pkgs ? import <nixpkgs> { } }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "harvest-code";
  version = "0.1.0";
  cargoLock.lockFile = ./Cargo.lock;
  cargoBuildFlags = [ "--bin" "translate" ];
  src = pkgs.lib.cleanSource ./.;
}

