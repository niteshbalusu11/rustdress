{ pkgs, lib, config, inputs, ... }: {
  packages = [
    pkgs.bacon
    pkgs.pkg-config
    pkgs.openssl
    pkgs.cmake
    pkgs.git
  ];

  languages.rust.enable = true;

  # Build the project
  scripts.build.exec = ''
    cargo build --release
  '';

  # Check the formatting of the project
  scripts.check-fmt.exec = ''
    cargo fmt --all -- --check
  '';

  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # Darwin-specific configurations
  env = lib.mkMerge [
    {
        env.OPENSSL_DIR = "${pkgs.openssl.dev}";
        env.OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
      # Common environment variables here
    }
    (lib.mkIf pkgs.stdenv.isDarwin {
      # Darwin-specific environment variables
      NIX_LDFLAGS = "-F${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -framework Security";
    })
  ];
}
