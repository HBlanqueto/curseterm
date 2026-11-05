{
  description = "CurseTerm: a terminal for Malaysia built on BarrierKit.";

  inputs = {
    barrierkit.url = "git+https://github.com/HBlanqueto/barrierkit?rev=ba58ce01ef1f8e07dd915f783290b6ef068c037a";
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
