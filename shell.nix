{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.clang
    pkgs.cmake
    pkgs.libllvm
    pkgs.libclang
    pkgs.pkg-config
    pkgs.libxml2
  ];

  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
  CMAKE_LLVM_DIR = "${pkgs.llvmPackages.libllvm.dev}/lib/cmake/llvm";
  CMAKE_CLANG_DIR = "${pkgs.llvmPackages.libclang.dev}/lib/cmake/clang";

}
