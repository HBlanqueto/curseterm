{
  description = "CurseTerm: a terminal for Malaysia built on BarrierKit.";

  inputs = {
    barrierkit.url = "git+https://github.com/HBlanqueto/barrierkit?rev=4e23c81353c68ec4da3333459b41d19eee7dd254";
    nixpkgs.follows = "barrierkit/nixpkgs";
    crane.follows = "barrierkit/crane";
  };

  outputs =
    { barrierkit, nixpkgs, crane, ... }:
    let
      systems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system: let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
        runtimeDeps = with pkgs; [
          libglvnd
          wayland
          libxkbcommon
          fontconfig
          freetype
          vulkan-loader
        ];
        craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.symlinkJoin {
          name = "rust-toolchain";
          paths = [ p.cargo p.rustc ];
        });
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./crates
            ./share
            ./Cargo.toml
            ./Cargo.lock
            ./LICENSE
          ];
        };
        commonArgs = {
          pname = "curseterm";
          version = "0.1.0";
          inherit src;
          strictDeps = true;
          cargoLock = ./Cargo.lock;
          env = {
            CURSETERM_DEV_SHELL = "1";
            SKIA_BINARIES_URL = "file://${barrierkit.packages.${system}.skia-binaries}/skia-binaries.tar.gz";
          };
          doCheck = false;
          nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
          buildInputs = runtimeDeps;
        };
        curseterm = craneLib.buildPackage (commonArgs // {
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          postInstall = ''
            install -Dm644 share/applications/org.curseterm.CurseTerm.desktop \
              $out/share/applications/org.curseterm.CurseTerm.desktop
            wrapProgram $out/bin/curseterm \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath runtimeDeps}"
          '';
          meta.mainProgram = "curseterm";
        });
      in {
        default = curseterm;
        inherit curseterm;
      });

      devShells = forAllSystems (system: {
        default = barrierkit.devShells.${system}.default.overrideAttrs (_: {
          CURSETERM_DEV_SHELL = "1";
        });
      });
    };
}
