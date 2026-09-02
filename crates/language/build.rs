fn main() {
    if let Ok(bundled) = std::env::var("CLAY_BUNDLE") {
        println!("cargo:rustc-env=ZED_BUNDLE={}", bundled);
    }
}
