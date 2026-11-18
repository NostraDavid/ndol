{
  pkgs,
  lib,
  ...
}: {
  cachix.enable = false;
  devenv.warnOnNewVersion = false;
  dotenv.enable = true;

  env = {
    CARGO_TERM_COLOR = "always";
    LOCALE_ARCHIVE = "${pkgs.glibcLocales}/lib/locale/locale-archive";
    PYTHONNOUSERSITE = "1";
    RUST_BACKTRACE = "1";
    UV_PYTHON_DOWNLOADS = "never";
    UV_PYTHON = "3.14";
    UV_PYTHON_PREFERENCE = "only-system";
    XDG_UTILS_INSTALL_MODE = "manual";
  };

  packages = with pkgs; [pkg-config];

  enterShell =
    (import (builtins.fetchurl {
        url = "https://raw.githubusercontent.com/NostraDavid/nixcfg/cb2423e55f4a7ee010d33a649ddf0488338baa42/nix/system-tools.nix";
        sha256 = "7d2abe646d9389e296c5e1cafa0b63dd52e9821c58037d9dd8faef90124104e2";
      }) (pkgs
        // {
          rustDev =
            (pkgs.buildEnv {
              name = "rust-dev-tools";
              paths = with pkgs; [rustc cargo clippy rustfmt rust-analyzer];
              pathsToLink = ["/bin"];
              postBuild = ''
                mkdir -p "$out/lib/rustlib/src/rust"
                ln -s ${pkgs.rustPlatform.rustLibSrc} "$out/lib/rustlib/src/rust/library"
              '';
            })
            // {version = pkgs.rustc.version;};
        }) [
        {
          attr = "rustDev";
          command = "rustc";
          series = lib.versions.majorMinor pkgs.rustc.version;
          also = ["cargo" "cargo-clippy" "rustfmt" "rust-analyzer"];
          check = ''
            local sysroot
            sysroot=$("$1" --print sysroot) && test -f "$sysroot/lib/rustlib/src/rust/library/core/src/lib.rs" &&
              test "$("$1" --version | cut -d' ' -f2 | cut -d. -f1,2)" = "$(cargo --version | cut -d' ' -f2 | cut -d. -f1,2)"'';
        }
        "just"
        "maturin"
        "python314"
        {
          attr = "uv";
          series = "0";
          also = ["uvx"];
        }
      ])
    + ''
      export RUST_SRC_PATH="$(dirname "$(type -P rustc)")/../lib/rustlib/src/rust/library"
      if [[ ! -f "$RUST_SRC_PATH/core/src/lib.rs" ]]; then
        export RUST_SRC_PATH="$(rustc --print sysroot)/lib/rustlib/src/rust/library"
      fi
    ''
    + ''
      unset PYTHONPATH
      unset PYTHONHOME
      unset _PYTHON_SYSCONFIGDATA_NAME

      if [ -d ".venv" ] && [ -x ".venv/bin/python" ]; then
        export VIRTUAL_ENV="$PWD/.venv"
        export PATH="$VIRTUAL_ENV/bin:$PATH"
      fi

      echo 'Protip: run `just` as your start point.' >&2
    '';
}
