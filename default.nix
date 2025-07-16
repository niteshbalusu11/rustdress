let
  nixpkgs = fetchTarball "https://github.com/nixos/nixpkgs/tarball/nixos-unstable";
  pkgs = import nixpkgs { config = { }; overlays = [ ]; };
in
{
  rustdress = pkgs.callPackage ./rustdress.nix { };
}
