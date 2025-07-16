{
  description = "Rustdress runs as a server and provides a Lighting Address as well as Nostr NIP-05 verification";

  inputs = {
    nixp0kgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { nixpkgs, ... }:
    let
      supportedSystems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = nixpkgs.legacyPackages;
    in
    {
      packages = forAllSystems (system: {
        default = pkgsFor.${system}.callPackage ./default.nix { };
      });
      devShells = forAllSystems (system: {
        default = pkgsFor.${system}.callPackage ./shell.nix { };
      });
    };
}
