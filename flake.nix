{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    utils.url = "github:numtide/flake-utils";
    oxalica-rust = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, nixpkgs, utils, oxalica-rust }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system}.extend oxalica-rust.overlays.default;
        rustStable = pkgs.rust-bin.stable.latest;
        rustNightly = pkgs.rust-bin.nightly.latest;
        rust = rustStable.minimal;

        unpack = drv: pkgs.runCommand "${drv.name}-unpacked" {} ''
          mkdir extracted
          cd extracted
          unpackFile ${drv}
          cp -a ./* $out
        '';

        ffmpeg = pkgs.ffmpeg_7;
      in {
        devShell = with pkgs; mkShell {
          nativeBuildInputs = with pkgs; [
            libxkbcommon
            libGL
            wayland
          ];
          buildInputs = [
            clang pkg-config
            openssl
            nasm
            rust-analyzer

            (enableDebugging ffmpeg).out
            (enableDebugging ffmpeg).dev

            (rust.override {
              extensions = [
                "rust-src"
                "clippy"
              ];
            })

            # necessary for unstable format options
            rustNightly.rustfmt

            (writeShellScriptBin "gdb" ''
              RUSTC_COMMIT="$(rustc -vV | awk '/commit-hash: (.+)/ { print $2 }')"

              exec ${gdb}/bin/gdb \
                -ex "set substitute-path '/rustc/$RUSTC_COMMIT' '${rustStable.rust-src}/lib/rustlib/src/rust'" \
                -d ${unpack ffmpeg.src} \
                "$@"
            '')
          ];

          shellHook = ''
          export LD_LIBRARY_PATH="${pkgs.libxkbcommon}/lib:${pkgs.libGL}/lib:${pkgs.wayland}/lib";
          export RUST_BACKTRACE=1;
        '';
          RUST_BACKTRACE = 1;
          LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
        };
      });
}
