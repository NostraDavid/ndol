{pkgs, ...}: {
  cachix.enable = false;
  devenv.warnOnNewVersion = false;
  dotenv.enable = true;

  env = {
    CARGO_TERM_COLOR = "always";
    LOCALE_ARCHIVE = "${pkgs.glibcLocales}/lib/locale/locale-archive";
    PYTHONNOUSERSITE = "1";
    RUST_BACKTRACE = "1";
    RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}/lib/rustlib/src/rust/library";
    UV_PYTHON_DOWNLOADS = "never";
    UV_PYTHON = "3.14";
    UV_PYTHON_PREFERENCE = "only-system";
    XDG_UTILS_INSTALL_MODE = "manual";
  };

  packages = with pkgs; [
    cargo
    clippy
    just
    maturin
    pkg-config
    python314
    rust-analyzer
    rustc
    rustfmt
    rustPlatform.rustLibSrc
    uv
  ];

  enterShell = ''
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
