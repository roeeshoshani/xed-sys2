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

    let xed_include_dir = format!("{}/include", xed_install_dir);
    let wrapped_static_fns_file_path = format!("{}/xed_wrapped_static_fns.c", out_dir);
    let bindings = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapped_static_fns_file_path.as_str())
        .clang_arg(format!("-I{}", xed_include_dir))
        .clang_arg("-DXED_ENCODER")
        .clang_arg("-DXED_DECODER")
        .rustified_enum("xed_machine_mode_enum_t")
        .rustified_enum("xed_address_width_enum_t")
        .use_core()
        .header("xed.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .unwrap();

    bindings
        .write_to_file(format!("{}/xed.rs", out_dir))
        .unwrap();

    cc::Build::new()
        .file(wrapped_static_fns_file_path.as_str())
        .flag("-DXED_ENCODER")
        .flag("-DXED_DECODER")
        .flag_if_supported("-Wno-duplicate-decl-specifier")
        .include(xed_include_dir.as_str())
        .include(cwd)
        .compile("xed_wrapped_static_fns");
}
