{
  description = "Nix development environment";

  # Flake inputs
  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay"; # a helper for Rust + Nix
    flake-utils.url  = "github:numtide/flake-utils";
  };

  # Flake outputs
  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
        let 
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          rustToolchain = pkgs.rust-bin.stable.latest.default;

        in 
        {
          devShells.default = pkgs.mkShell {
	  nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs =  [
              rustToolchain
	      pkgs.openssl
	      pkgs.openssl.dev
            ]; 

            shellHook = ''
            '';
          };
        }
    );
}
