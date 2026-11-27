use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Embedded by `sqlx::migrate!`, which doesn't notice new migration files by itself.
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=migrations_sutom");
    println!("cargo:rerun-if-changed=migrations_calendar");
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/vite.config.ts");

    let frontend_dir = Path::new("frontend");
    let dist_dir = frontend_dir.join("dist");
    let resources_dir = Path::new("public");

    let skip = std::env::var("SKIP_FRONTEND_BUILD").is_ok();

    if skip {
        assert!(
            dist_dir.exists(),
            "SKIP_FRONTEND_BUILD is set but frontend/dist/ does not exist — \
             copy the pre-built frontend before running cargo build"
        );
    } else {
        let install = Command::new("npm")
            .args(["install"])
            .current_dir(frontend_dir)
            .status()
            .expect("Failed to run npm install");
        assert!(install.success(), "npm install failed");

        let build = Command::new("npm")
            .args(["run", "build"])
            .current_dir(frontend_dir)
            .status()
            .expect("Failed to run npm run build");
        assert!(build.success(), "npm run build failed");
    }

    if resources_dir.exists() {
        fs::remove_dir_all(resources_dir).expect("Failed to clean resources/");
    }
    copy_dir(&dist_dir, resources_dir).expect("Failed to copy dist to resources/");
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
