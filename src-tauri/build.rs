fn main() {
    println!("cargo:rerun-if-changed=../src/control.html");
    println!("cargo:rerun-if-changed=../src/notch.html");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}