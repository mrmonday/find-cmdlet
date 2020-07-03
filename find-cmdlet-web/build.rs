use anyhow::{anyhow, Context};
use ructe::Ructe;
use std::env;
use std::{path::Path, process::Command};

fn compile_typescript(out_dir: &str) -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=typescript/tsconfig.json");
    println!("cargo:rerun-if-changed=typescript/index.ts");

    let status = Command::new("tsc")
        .arg("--project")
        .arg("typescript/tsconfig.json")
        .arg("--outFile")
        .arg(Path::new(out_dir).join("index.js"))
        .status()
        .context("Failed to execute TypeScript compiler")?;

    if !status.success() {
        return Err(anyhow!("TypeScript compile failed: {:?}", status));
    }

    Ok(())
}

fn uglify_javascript(out_dir: &str, statics_dir: &str) -> anyhow::Result<()> {
    let input_js = Path::new(out_dir).join("index.js");
    let input_js_map = Path::new(out_dir).join("index.js.map");
    let output_js = Path::new(statics_dir).join("index.min.js");
    println!("cargo:rerun-if-changed={}", input_js.to_string_lossy());
    println!("cargo:rerun-if-changed={}", input_js_map.to_string_lossy());

    let status = Command::new("terser")
        .arg(input_js)
        .arg("--source-map")
        .arg(format!(
            "content='{}',url='{}'",
            input_js_map.to_string_lossy(),
            "index.min.js.map"
        ))
        .arg("--compress")
        .arg("--toplevel")
        .arg("--mangle")
        .arg("--output")
        .arg(output_js)
        .status()
        .context("Failed to execute terser")?;

    if !status.success() {
        return Err(anyhow!("Uglification failed: {:?}", status));
    }

    Ok(())
}

fn compile_sass(statics_dir: &str) -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=sass/style.scss");

    let status = Command::new("sass")
        .arg("-s")
        .arg("compressed")
        .arg("sass/style.scss")
        .arg(Path::new(statics_dir).join("style.css"))
        .status()
        .context("Failed to execute Sass")?;

    if !status.success() {
        return Err(anyhow!("Sass compile failed: {:?}", status));
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let out_dir = env::var("OUT_DIR")?;
    let static_dir = "static";

    compile_typescript(&out_dir)?;
    uglify_javascript(&out_dir, static_dir)?;
    compile_sass(static_dir)?;

    let mut ructe = Ructe::from_env()?;

    ructe.compile_templates("templates")?;

    Ok(())
}
