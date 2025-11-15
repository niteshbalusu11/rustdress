{
  description = "Rustdress: Self hosted Lightning Address Server and NIP-05 Verification Server";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs =
    {
      flake-utils,
      nixpkgs,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            bacon
            cargo
            clippy
            cmake
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
