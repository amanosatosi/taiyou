fn main() {
    let target = std::env::var("TARGET").expect("Cargo supplies TARGET");
    if target.contains("linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    }
}
