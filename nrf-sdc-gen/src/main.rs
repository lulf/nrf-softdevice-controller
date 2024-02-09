//! Calls out to bindgen to generate a Rust crate from the Nordic header
//! files.

fn main() {
    use std::path::PathBuf;
    let nrfxlib_path = "../sdk-nrfxlib";
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Our own headers
        .clang_arg(format!("-I{}", nrfxlib_path))
        // Point to Nordic headers
        .clang_arg(format!("-I./include"))
        .clang_arg(format!("-DPPI_PRESENT"))
        .clang_arg(format!("-I{}/softdevice_controller/include", nrfxlib_path))
        .clang_arg(format!("-I{}/mpsl/include", nrfxlib_path))
        // Point to our special local headers
        // Add extra paths that the C files assume are searched
        //.clang_arg("-I../../sdk-nrfxlib/crypto/nrf_cc310_platform/include")
        //.clang_arg("-I../../sdk-nrfxlib/crypto/nrf_oberon")
        // Disable standard includes (they belong to the host)
        .clang_arg("-nostdinc")
        // Set the target
        .clang_arg("-target")
        .clang_arg("arm")
        .clang_arg("-mcpu=cortex-m4")
        // Use softfp
        .clang_arg("-mfloat-abi=hard")
        // We're no_std
        .use_core()
        .ctypes_prefix("crate::ctypes")
        // Include only the useful stuff
        // .allowlist_function("nrf_.*")
        // .allowlist_function("sdc.*")
        // .allowlist_function("mpsl.*")
        //.allowlist_type("nrf_.*")
        //.allowlist_type("sdc.*")
        // .allowlist_var("NRF_.*")
        // .allowlist_var("SDC_.*")
        // .allowlist_var("MPSL_.*")
        //.allowlist_var("BSD_.*")
        //.allowlist_var("OCRYPTO_.*")
        // Format the output
        .formatter(bindgen::Formatter::Rustfmt)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let mut rust_source = bindings.to_string();

    // Munge Doxygen comments into something Rustdoc can handle
    rust_source = rust_source.replace("#[doc = \"@{*/\"]", "");
    let re = regex::Regex::new("\"   \\s+- ").unwrap();
    rust_source = re.replace_all(&rust_source, "\" * ").into();
    let re = regex::Regex::new(r"\s*@param\s+(?P<var>[A-Za-z0-9_]+)\s+").unwrap();
    rust_source = re.replace_all(&rust_source, " * `$var` - ").into();
    let re = regex::Regex::new(r"\s*@param\[(out|in|inout|in,out)\](\\t|\s+)(?P<var>[A-Za-z0-9_]+)\s+").unwrap();
    rust_source = re.replace_all(&rust_source, " * `$var` - ").into();
    let re = regex::Regex::new(r"@[cp]\s+(?P<var>[A-Za-z0-9_\(\)]+)").unwrap();
    rust_source = re.replace_all(&rust_source, " * `$var` - ").into();
    let re = regex::Regex::new(r"\\\\[cp]\s+(?P<var>[A-Za-z0-9_\(\)]+)").unwrap();
    rust_source = re.replace_all(&rust_source, "`$var`").into();
    let re = regex::Regex::new(r"\\\\ref\s+(?P<var>[A-Za-z0-9_\(\)]+)").unwrap();
    rust_source = re.replace_all(&rust_source, "`$var`").into();
    rust_source = rust_source.replace("\" @remark", "\" NB: ");
    rust_source = rust_source.replace("\"@brief", "\"");
    rust_source = rust_source.replace("\" @brief", "\" ");
    rust_source = rust_source.replace("\"@detail", "\"");
    rust_source = rust_source.replace("\" @detail", "\" ");
    rust_source = rust_source.replace("@name ", "# ");
    rust_source = rust_source.replace("@return ", "Returns ");
    rust_source = rust_source.replace("@retval ", "Returns ");

    let bindings_out_path = PathBuf::from("../nrf-sdc-sys/src").join("bindings.rs");
    std::fs::write(bindings_out_path, rust_source).expect("Couldn't write updated bindgen output");
}
