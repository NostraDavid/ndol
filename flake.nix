{
  description = "ndol development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";
    devenv.url = "github:cachix/devenv";
  };

  outputs = inputs @ {
    nixpkgs,
    devenv,
    ...
  }: let
    inherit (nixpkgs) lib;
    systems = ["x86_64-linux"];
    forAllSystems = f: lib.genAttrs systems f;
  in {
    devShells = forAllSystems (system: {
      default = devenv.lib.mkShell {
        inherit inputs system;
        modules = [
          ./devenv.nix
        ];
      };
    });
  };
}
