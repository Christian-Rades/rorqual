{
  pkgs ? (import <nixpkgs>){}
}:
with pkgs;

rustPlatform.buildRustPackage rec {
  pname = "rorqual";
  version = "0.0.1";

  nativeBuildInputs=[ rust-analyzer cargo-flamegraph rustfmt clippy openssl openssl.dev pkg-config];
  buildInputs = [
    rustfmt
    clippy
    
    openssl
    openssl.dev
    libgit2
  ];

  src = ./.;

  cargoSha256 = "sha256-CNe8ljVebCu6n71pa4Y+OoOJfE78qnSYZJMhfRbRGKk=";
}
