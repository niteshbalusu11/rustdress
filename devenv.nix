{ pkgs, lib, config, inputs, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = [ 
    pkgs.bacon
    pkgs.pkg-config
    pkgs.openssl
    pkgs.cmake
  ];

  # https://devenv.sh/languages/
  languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  scripts.build.exec = ''
    bacon 
  '';

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';


  env.OPENSSL_DIR = "${pkgs.openssl.dev}";
  env.OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

  # Include the Security framework
  env.NIX_LDFLAGS = "-F${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -framework Security";
  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
