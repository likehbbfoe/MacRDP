fn main() {
    println!("cargo:rerun-if-env-changed=SWIFT_LIB_PATH");
    // The workspace links the macOS 14+ system Swift runtime. Keep an explicit
    // developer override without probing files that may live in the dyld cache.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        if let Ok(path) = std::env::var("SWIFT_LIB_PATH") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{path}");
        }
    }
}
