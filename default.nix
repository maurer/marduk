with import <nixpkgs> {};
((import ./marduk.nix).marduk {}).override {
  crateOverrides = defaultCrateOverrides // {
    bap-sys = attrs: {
      buildInputs = [ libbap llvmPackages.clang-unwrapped.lib clang ];
      LIBCLANG_PATH = "${llvmPackages.clang-unwrapped.lib}/lib";
    };
    marduk = attrs: {
      buildInputs = [ libbap ];
    };
  };
}
