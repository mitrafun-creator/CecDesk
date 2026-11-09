fn main() {
    let mut win_attrs = tauri_build::WindowsAttributes::new();
    if std::path::Path::new("C:/rust_target/icon.ico").exists() {
        win_attrs = win_attrs.window_icon_path("C:/rust_target/icon.ico");
    }
    let attrs = tauri_build::Attributes::new().windows_attributes(win_attrs);
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
