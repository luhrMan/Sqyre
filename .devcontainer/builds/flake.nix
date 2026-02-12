# Build Sqyre on NixOS so the binary is linked against the Nix store and runs
# natively. Use this instead of running the devcontainer-built binary on NixOS.
#
# Usage:
#   nix develop
#   mkdir -p out && go build -o out/sqyre ./cmd/sqyre
#   ./out/sqyre

{
  description = "Sqyre macro builder";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  outputs = { self, nixpkgs }: let
    systems = [ "x86_64-linux" "aarch64-linux" ];
    forAllSystems = nixpkgs.lib.genAttrs systems;
    pkgsFor = system: import nixpkgs { inherit system; };
    mkShell = system: let pkgs = pkgsFor system; in pkgs.mkShell {
      name = "sqyre-build";
      buildInputs = with pkgs; [
        go
        # build-essential equivalent
        gcc
        pkg-config
        # OpenGL / Fyne
        libGL
        libglvnd
        glfw
        libxkbcommon
        # X11
        xorg.libX11
        xorg.libxcb
        xorg.libXext
        xorg.libXi
        xorg.libXtst
        xorg.libXcursor
        xorg.libXrandr
        xorg.libXinerama
        xorg.libXxf86vm
        xorg.libXt
        # OpenCV, Tesseract, Leptonica
        opencv4
        tesseract
        leptonica
      ];
      PKG_CONFIG_PATH = "${pkgs.opencv4}/lib/pkgconfig";
      CGO_ENABLED = "1";
      GOFLAGS = "-tags=gocv_specific_modules";
    };
  in {
    devShells = forAllSystems (system: { default = mkShell system; });
  };
}
