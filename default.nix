{
  pkgs ? (import <nixpkgs>){}
}:
with pkgs;

rustPlatform.buildRustPackage rec {
  pname = "rorqual";
  version = "0.0.1";

  nativeBuildInputs=[ rust-analyzer rustfmt clippy openssl openssl.dev pkg-config];
  buildInputs = [
    rustfmt
    clippy
    
    openssl
    openssl.dev
    libgit2
  ];

  src = ./.;

  cargoSha256 = "16hn66h1v2mq0mhk7npmdrsrk8i4l8zpcwzrbqmlplpav7y2hsj5";
}
