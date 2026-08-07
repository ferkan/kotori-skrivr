{
  description = "Kotori Skrivr - a fast, lightweight text editor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ]
      (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          lib = pkgs.lib;
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

          linuxBuildInputs = with pkgs; [
            gtk3
            libxkbcommon
            wayland
            vulkan-loader
            libGL
            libx11
            libxcursor
            libxi
            libxrandr
            libxcb
          ];

          darwinFrameworks = with pkgs.darwin.apple_sdk.frameworks; [
            AppKit
            Cocoa
            CoreFoundation
            CoreGraphics
            CoreServices
            Foundation
            Metal
            QuartzCore
          ];

          skrivr = pkgs.rustPlatform.buildRustPackage {
            pname = "skrivr";
            version = cargoToml.package.version;

            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkgs.pkg-config ]
              ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.wrapGAppsHook3 ];

            buildInputs = lib.optionals pkgs.stdenv.hostPlatform.isLinux linuxBuildInputs
              ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin darwinFrameworks;

            doCheck = false;

            meta = with lib; {
              description = "A fast, lightweight text editor for Markdown, JSON, and more";
              homepage = "https://github.com/OlaProeis/Kotori Skrivr";
              license = licenses.mit;
              mainProgram = "skrivr";
              platforms = platforms.linux ++ platforms.darwin;
            };
          };
        in
        {
          packages = {
            default = skrivr;
            skrivr = skrivr;
          };

          apps.default = {
            type = "app";
            program = "${skrivr}/bin/skrivr";
          };

          devShells.default = pkgs.mkShell {
            packages = [
              pkgs.rustc
              pkgs.cargo
              pkgs.rustfmt
              pkgs.clippy
              pkgs.rust-analyzer
              pkgs.pkg-config
            ]
            ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux linuxBuildInputs
            ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin darwinFrameworks;

            shellHook = ''
              echo "Kotori Skrivr Nix dev shell ready."
              echo "Run cargo commands normally, e.g. cargo build --release"
            '';
          };

          checks.default = skrivr;
        }
      );
}
