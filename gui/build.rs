fn main() {
    // libacercontrol is installed by meson; the exact libdir varies by distro.
    for dir in [
        "/usr/local/lib64",
        "/usr/local/lib",
        "/usr/local/lib/x86_64-linux-gnu",
        "/usr/lib64",
        "/usr/lib",
    ] {
        println!("cargo:rustc-link-search=native={}", dir);
    }

    println!("cargo:rustc-link-lib=acercontrol");
}
