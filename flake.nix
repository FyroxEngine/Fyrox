{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }:
  let
    overlays = [
      (import rust-overlay)
    ];

    systems = [
      "x86_64-linux"
      "aarch64-linux"
    ];

    forAllSystems = f:
      nixpkgs.lib.genAttrs systems
      (system: f { pkgs = import nixpkgs { inherit system overlays; }; });
  in
  {
    devShells = forAllSystems ({ pkgs }: with pkgs; {
      default = mkShell rec {
        nativeBuildInputs = [
          pkg-config
          rust-bin.stable.latest.default
        ];

        buildInputs = [
          libx11
          libxcb
          alsa-lib
          wayland
          libxkbcommon
          libGL
        ];

        LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;

        # Remove this if you don't need LSP support
        RUST_SRC_PATH = "${rust-bin.stable.latest.rust-src}/lib/rustlib/src/rust/library";
      };
    });
  };
}
