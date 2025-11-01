{
  description = "Rustdress: Self hosted Lightning Address Server and NIP-05 Verification Server";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nixpkgs-cmake-3-29.url = "github:nixos/nixpkgs/d4f247e89f6e10120f911e2e2d2254a050d0f732";

  outputs =
    { flake-utils
    , nixpkgs
    , nixpkgs-cmake-3-29
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        pkgs-cmake-3-29 = nixpkgs-cmake-3-29.legacyPackages.${system};
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            bacon
            cargo
            clippy
            pkgs-cmake-3-29.cmake
            git
            openssl
            pkg-config
            rust-analyzer
            rustc
            rustfmt
          ];
        };
      }
    );
}
