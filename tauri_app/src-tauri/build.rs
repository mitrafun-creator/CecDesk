fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");

    let mut win_attrs = tauri_build::WindowsAttributes::new();
    let repo_icon = std::path::Path::new("icons/icon.ico");
    if repo_icon.is_file() {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let out_icon = std::path::Path::new(&out_dir).join("app_icon.ico");
            if std::fs::copy(repo_icon, &out_icon).is_ok() {
                win_attrs = win_attrs.window_icon_path(out_icon);
            } else {
                win_attrs = win_attrs.window_icon_path(repo_icon);
            }
        } else {
            win_attrs = win_attrs.window_icon_path(repo_icon);
        }
    } else if std::path::Path::new("C:/rust_target/icon.ico").exists() {
        win_attrs = win_attrs.window_icon_path("C:/rust_target/icon.ico");
    }
    let attrs = tauri_build::Attributes::new().windows_attributes(win_attrs);
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
