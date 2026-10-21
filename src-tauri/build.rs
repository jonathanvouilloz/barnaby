fn main() {
    // sherpa-onnx ships as shared libraries: next to the binary in dev, in /usr/lib/barnaby
    // once installed (deb/AppImage, see tauri.linux.conf.json).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN:$ORIGIN/../lib/barnaby");
    }
    tauri_build::build()
}
