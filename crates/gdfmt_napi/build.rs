fn main() {
    napi_build::setup();
    println!(
        "cargo:rustc-env=GDFMT_NATIVE_TARGET={}",
        std::env::var("TARGET").expect("Cargo target")
    );
}
