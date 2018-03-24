with import <nixpkgs> {};

stdenv.mkDerivation {
	name = "tokio-coap";
	buildInputs = [
		gcc
		gdb
	];
}
