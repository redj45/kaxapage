use std::{fs, path::Path};

fn main() {
    // Rebuild if these change (useful even if admin-dist is generated)
    println!("cargo:rerun-if-changed=web/admin/package.json");
    println!("cargo:rerun-if-changed=web/admin/package-lock.json");
    println!("cargo:rerun-if-changed=web/admin/vite.config.ts");

    // Rebuild if embedded assets change
    let dir = Path::new("web/admin-dist");
    watch_dir_recursive(dir);
}

fn watch_dir_recursive(dir: &Path) {
    if !dir.exists() {
        // If dist doesn't exist yet, still re-run when the directory itself appears/changes
        println!("cargo:rerun-if-changed={}", dir.display());
        return;
    }

    if dir.is_file() {
        println!("cargo:rerun-if-changed={}", dir.display());
        return;
    }

    // Directory: walk all entries and register each file
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            println!("cargo:rerun-if-changed={}", dir.display());
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            watch_dir_recursive(&path);
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
