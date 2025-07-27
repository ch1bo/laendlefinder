{
  description = "LeandleFinder - A scraper for real estate sales data in Vorarlberg";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfreePredicate = pkg: builtins.elem (pkgs.lib.getName pkg) [
            "claude-code"
          ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };
      in
      {
        packages.default = ./web;

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            claude-code
            rustToolchain
            pkg-config
            openssl
            openssl.dev
            simple-http-server
          ];

          shellHook = ''
            echo "LeandleFinder Development Environment"
            echo "--------------------------------------"
            echo "Run 'cargo run' to start the scraper"
            echo "Run 'simple-http-server web -i -o' to start the visualization"
          '';

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          # Set SSL certificate path for reqwest
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };
      }
    );
}
