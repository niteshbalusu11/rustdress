{ lib, fetchFromGitHub, rustPlatform, bacon, cmake, openssl, pkg-config }:
# { lib, fetchFromGitHub, pkgs ? import <nixpkgs> { } }:

rustPlatform.buildRustPackage rec {
  pname = "rustdress";
  version = "0.5.2";

  src = fetchFromGitHub {
    owner = "niteshbalusu11";
    repo = pname;
    rev = "v${version}";
    sha256 = "sha256-XEXvAAnktr7gfk3y8kLtrVmg0slx5wc4dCCWT2r+Wj0=";
  };

  # cargoHash = "0000000000000000000000000000000000000000000000000000";
  cargoLock.lockFile = ./Cargo.lock;

  buildInputs = [
    bacon
    cmake
    # git
    openssl
    pkg-config
  ];

  # doCheck = false;

  meta = with lib; {
    description = "Self hosted Lightning Address Server";
    homepage = "https://github.com/${owner}/${pname}";
    license = licenses.mit;
    platforms = platforms.unix;
  };
}
