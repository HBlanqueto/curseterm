{
  description = "CurseTerm: a terminal for Malaysia built on BarrierKit.";

  inputs = {
    barrierkit.url = "git+https://github.com/HBlanqueto/barrierkit?rev=4e23c81353c68ec4da3333459b41d19eee7dd254";
    nixpkgs.follows = "barrierkit/nixpkgs";
  };

  outputs =
    { barrierkit, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" ];
    in
    {
      devShells = nixpkgs.lib.genAttrs systems (system: {
        default = barrierkit.devShells.${system}.default.overrideAttrs (_: {
          CURSETERM_DEV_SHELL = "1";
        });
      });
    };
}
