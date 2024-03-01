use std::{env::current_dir, fs::create_dir_all, process::Command};

use bindgen::callbacks::ParseCallbacks;

/// custom parser callbacks which implement passthrough to `CargoCallbacks` but avoid emitting `rerun-if-changed` entries for files that
/// are generated by the build script, otherwise cargo will think that those files are stale every single build and will thus re-run this
/// build script every single time, which is undesirable.
#[derive(Debug)]
struct XedBindgenParseCallbacks {
    xed_install_include_dir: String,
    cargo_callbacks: bindgen::CargoCallbacks,
}
impl ParseCallbacks for XedBindgenParseCallbacks {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        self.cargo_callbacks.will_parse_macro(name)
    }

    fn generated_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        self.cargo_callbacks.generated_name_override(item_info)
    }

    fn generated_link_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        self.cargo_callbacks.generated_link_name_override(item_info)
    }

    fn int_macro(&self, name: &str, value: i64) -> Option<bindgen::callbacks::IntKind> {
        self.cargo_callbacks.int_macro(name, value)
    }

    fn str_macro(&self, name: &str, value: &[u8]) {
        self.cargo_callbacks.str_macro(name, value)
    }

    fn func_macro(&self, name: &str, value: &[&[u8]]) {
        self.cargo_callbacks.func_macro(name, value)
    }

    fn enum_variant_behavior(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<bindgen::callbacks::EnumVariantCustomBehavior> {
        self.cargo_callbacks
            .enum_variant_behavior(enum_name, original_variant_name, variant_value)
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        self.cargo_callbacks
            .enum_variant_name(enum_name, original_variant_name, variant_value)
    }

    fn item_name(&self, original_item_name: &str) -> Option<String> {
        self.cargo_callbacks.item_name(original_item_name)
    }

    fn header_file(&self, filename: &str) {
        self.cargo_callbacks.header_file(filename)
    }

    fn include_file(&self, filename: &str) {
        if filename.starts_with(self.xed_install_include_dir.as_str()) {
            return;
        }
        self.cargo_callbacks.include_file(filename)
    }

    fn read_env_var(&self, key: &str) {
        self.cargo_callbacks.read_env_var(key)
    }

    fn blocklisted_type_implements_trait(
        &self,
        name: &str,
        derive_trait: bindgen::callbacks::DeriveTrait,
    ) -> Option<bindgen::callbacks::ImplementsTrait> {
        self.cargo_callbacks
            .blocklisted_type_implements_trait(name, derive_trait)
    }

    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        self.cargo_callbacks.add_derives(info)
    }

    fn process_comment(&self, comment: &str) -> Option<String> {
        self.cargo_callbacks.process_comment(comment)
    }

    fn field_visibility(
        &self,
        info: bindgen::callbacks::FieldInfo<'_>,
    ) -> Option<bindgen::FieldVisibilityKind> {
        self.cargo_callbacks.field_visibility(info)
    }

    fn wrap_as_variadic_fn(&self, name: &str) -> Option<String> {
        self.cargo_callbacks.wrap_as_variadic_fn(name)
    }
}

fn main() {
    let cwd_path = current_dir().unwrap();
    let cwd = cwd_path.to_str().unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let xed_build_dir = format!("{}/xed", out_dir);
    let xed_install_dir = format!("{}/xed_install", out_dir);
    let num_jobs_str = std::env::var("NUM_JOBS").unwrap_or_else(|_| "1".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    println!("cargo:rerun-if-changed-env=OUT_DIR");
    println!("cargo:rerun-if-changed-env=PROFILE");

    create_dir_all(xed_build_dir.as_str()).unwrap();

    let mut cmd = Command::new(format!("{}/xed/mfile.py", cwd));
    cmd.arg("-j")
        .arg(num_jobs_str.as_str())
        .arg("--static")
        .arg("--enc2")
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
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .use_core()
        .header("xed.h")
        .parse_callbacks(Box::new(XedBindgenParseCallbacks {
            xed_install_include_dir: xed_include_dir.clone(),
            cargo_callbacks: bindgen::CargoCallbacks::new(),
        }))
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
