{
  description = "srvcs-crossproduct: vectors: 3D cross product";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        version = "0.1.0";
        rustToolchain = pkgs.rust-bin.stable."1.96.0".default.override {
          extensions = [ "clippy" "rustfmt" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        packages = {
          default = rustPlatform.buildRustPackage {
            pname = "srvcs-crossproduct";
            inherit version;
            src = ./.;
            cargoHash = "sha256-e/P/hB+jzVxI6mbaXN6T0N1j8Mro6JtmpqBcbPrxwCE=";
          };
        } // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          container = pkgs.dockerTools.buildLayeredImage {
            name = "srvcs-crossproduct";
            tag = "latest";
            config = {
              Entrypoint = [ "${self.packages.${system}.default}/bin/srvcs-crossproduct" ];
              ExposedPorts = { "8080/tcp" = { }; };
              User = "65534:65534";
              Labels = {
                "org.opencontainers.image.title" = "srvcs-crossproduct";
                "org.opencontainers.image.description" = "3D cross product orchestrator: composes srvcs-floatmultiply and srvcs-floatsubtract to compute the vector cross product a × b.";
                "org.opencontainers.image.version" = version;
                "org.opencontainers.image.revision" = self.rev or "dev";
                "org.opencontainers.image.source" = "https://github.com/srvcs/crossproduct";
                "org.opencontainers.image.licenses" = "Apache-2.0";
              };
            };
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [ rustToolchain pkgs.syft ];
        };
      });
}
