{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustup

    # Needed for c2rust/llvm dependencies
    pkgs.clang
    pkgs.cmake
    pkgs.libllvm
    pkgs.libclang
    pkgs.pkg-config
    pkgs.libxml2
    pkgs.git
    pkgs.cacert
  ];

  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
  CMAKE_LLVM_DIR = "${pkgs.llvmPackages.libllvm.dev}/lib/cmake/llvm";
  CMAKE_CLANG_DIR = "${pkgs.llvmPackages.libclang.dev}/lib/cmake/clang";

  # For LLVM AST Parsing, needs to point to _some_ libc headers
  C_INCLUDE_PATH = "${pkgs.musl.dev}/include/";

}
