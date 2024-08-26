fn main() {
    println!(
        "cargo::rustc-env=COMMONIZE_OUT_DIR={}",
        std::env::var("OUT_DIR").unwrap_or("".to_owned())
    );
}
