fn main() {
    println!("cargo:rustc-link-search=native={}", ".");
    println!("cargo:rustc-link-lib=static=power_hardabi");
}