fn main() {
    println!(
        "cargo::rustc-env=COMMONIZE_OUT_DIR={}",
        std::env::var("OUT_DIR").unwrap_or("".to_owned())
    );
    println!(
        "cargo::rustc-env=COMMONIZE_ENV={}{}{}{}",
        std::env::var("TARGET").unwrap_or("".to_owned()),
        std::env::var("HOST").unwrap_or("".to_owned()),
        std::env::var("OPT_LEVEL").unwrap_or("".to_owned()),
        std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or("".to_owned())
    );
}
