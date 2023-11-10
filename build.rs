use std::{env::current_dir, fs::create_dir_all, process::Command};

fn main() {
    let cwd_path = current_dir().unwrap();
    let cwd = cwd_path.to_str().unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let xed_build_dir = format!("{}/xed", out_dir);
    let xed_install_dir = format!("{}/xed_install", out_dir);
    let num_jobs_str = std::env::var("NUM_JOBS").unwrap_or_else(|_| "1".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    println!("cargo:rerun-if-changed={}/xed", cwd);
    println!("cargo:rerun-if-changed={}/mbuild", cwd);
    println!("cargo:rerun-if-changed-env=OUT_DIR");
    println!("cargo:rerun-if-changed-env=PROFILE");

    create_dir_all(xed_build_dir.as_str()).unwrap();

    let mut cmd = Command::new(format!("{}/xed/mfile.py", cwd));
    cmd.arg("-j")
        .arg(num_jobs_str.as_str())
        .arg("--static")
        .arg(format!("--install-dir={}", xed_install_dir))
        .current_dir(xed_build_dir.as_str());

    if profile == "release" {
        cmd.arg("--opt=3");
    } else {
        cmd.arg("--opt=0");
    }

    cmd.arg("install");

    let exit_code = cmd.spawn().unwrap().wait().unwrap();
    assert!(exit_code.success());

    println!("cargo:rustc-link-search=native={}/lib", xed_install_dir);
    println!("cargo:rustc-link-lib=static=xed");

    let bindings = bindgen::Builder::default()
        .clang_arg(format!("--include-directory={}/include", xed_install_dir))
        .header("xed.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .unwrap();
    bindings
        .write_to_file(format!("{}/xed.rs", out_dir))
        .unwrap();
}
