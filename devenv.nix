{ pkgs, lib, config, inputs, ... }: {
  packages = [
    pkgs.bacon
    pkgs.pkg-config
    pkgs.openssl
    pkgs.cmake
  ];

  languages.rust.enable = true;

  scripts.build.exec = ''bacon'';

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
