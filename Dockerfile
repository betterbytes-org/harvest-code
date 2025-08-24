# Base Image
FROM nixos/nix

USER root
RUN nix-env -iA nixpkgs.rustup
RUN rustup default stable