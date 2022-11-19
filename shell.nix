{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ rustup cargo gcc ];
  buildInputs = with pkgs; [ 
    clang 
    rustfmt 
    clippy 
    just 
    kube3d 
    openssl 
    pkgconfig 
    python310Packages.ipython
    protobuf
    kustomize
    kubectl
  ];

  # Certain Rust tools won't work without this
  # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
  # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  # This is required to build hun-spell for cargo-spellchecker
  LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"; 

  # TODO: use cargo bin path env varibles instead of specific path.
  shellHook = ''
    export PATH=$PATH:~/.cargo/bin
  '';
}
