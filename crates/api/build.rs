use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[path = "src/embed_policy.rs"]
mod embed_policy;

use embed_policy::{decide, EmbedChoice};

const HTML_SHELL: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Taskboard</title>
</head>
<body>
  <div id="root"></div>
</body>
</html>
"#;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("web-dist");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vite_dist = manifest.join("../../apps/web/dist");
    println!("cargo:rerun-if-changed={}", vite_dist.display());
    println!(
        "cargo:rerun-if-changed={}",
        vite_dist.join("index.html").display()
    );

    let profile = env::var("PROFILE").unwrap_or_default();
    let index = vite_dist.join("index.html");
    match decide(&profile, index.is_file()) {
        Ok(EmbedChoice::CopyDist) => copy_dir(&vite_dist, &out).expect("copy vite dist"),
        Ok(EmbedChoice::HtmlShell) => {
            println!(
                "cargo:warning=apps/web/dist/index.html missing; embedding the blank HTML shell"
            );
            fs::write(out.join("index.html"), HTML_SHELL).unwrap();
        }
        Err(message) => panic!("{message}"),
    }
}

fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}
