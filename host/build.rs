fn main() {
    // screencapturekit ships a Swift bridge; the Swift runtime lives in /usr/lib/swift.
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}
