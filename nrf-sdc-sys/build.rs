//! Calls out to bindgen to generate a Rust crate from the Nordic header
//! files.

fn main() {
    use std::env;
    use std::path::{Path, PathBuf};
    let nrfxlib_path = "../sdk-nrfxlib";

    let libsoftdevice_controller_original_path = Path::new(&nrfxlib_path)
        .join("softdevice_controller/lib/cortex-m4/hard-float/libsoftdevice_controller_multirole.a");
    let libsoftdevice_controller_changed_path =
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("libsoftdevice_controller.a");
    // The softdevice_controller library now has compressed headers, but Rust cannot deal with that.
    // If the appropriate features is active, we're gonna strip it or decompress it.
    #[cfg(feature = "arm-none-eabi-objcopy")]
    {
        // We assume the arm-none-eabi-objcopy comes from the official arm website.
        let child = std::process::Command::new("arm-none-eabi-objcopy")
            .arg("--decompress-debug-sections")
            .arg(&libsoftdevice_controller_original_path)
            .arg(&libsoftdevice_controller_changed_path)
            .spawn()
            .expect("Could not start `arm-none-eabi-objcopy`. Is it installed and available in your path?");

        let child_result = child.wait_with_output().unwrap();
        if !child_result.status.success() {
            panic!("Something went wrong with `arm-none-eabi-objcopy`.");
        }
    }

    #[cfg(feature = "llvm-objcopy")]
    {
        // We assume the llvm-objcopy comes from the rustup llvm-tools.
        // This cannot do decompression, so we'll just strip the debug sections

        let tool_error = "Could not find `llvm-objcopy`. Is it installed? Use `rustup component add llvm-tools` to install it or select the `arm-none-eabi-objcopy` feature if you have that tool installed.";
        // It's not in our path, so we have to search for it
        let path = llvm_tools::LlvmTools::new()
            .expect(tool_error)
            .tool(&llvm_tools::exe("llvm-objcopy"))
            .expect(tool_error);

        let child = std::process::Command::new(path)
            .arg("--strip-debug")
            .arg(&libsoftdevice_controller_original_path)
            .arg(&libsoftdevice_controller_changed_path)
            .spawn()
            .expect(tool_error);

        let child_result = child.wait_with_output().unwrap();
        if !child_result.status.success() {
            panic!("Something went wrong with `llvm-objcopy`.");
        }
    }

    // Make sure we link against the libraries
    println!(
        "cargo:rustc-link-search={}",
        libsoftdevice_controller_changed_path.parent().unwrap().display()
    );
    //println!(
    //	"cargo:rustc-link-search={}",
    //	Path::new(&nrfxlib_path)
    //		.join("crypto/nrf_oberon/lib/cortex-m33/hard-float")
    //		.display()
    //);
    println!("cargo:rustc-link-lib=static=softdevice_controller");
    //	println!("cargo:rustc-link-lib=static=oberon_3.0.13");
    //
    // MPSL
    let libmpsl_original_path = Path::new(&nrfxlib_path).join("mpsl/lib/cortex-m4/hard-float/libmpsl.a");
    let libmpsl_changed_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("libmpsl.a");

    // The softdevice_controller library now has compressed headers, but Rust cannot deal with that.
    // If the appropriate features is active, we're gonna strip it or decompress it.
    #[cfg(feature = "arm-none-eabi-objcopy")]
    {
        // We assume the arm-none-eabi-objcopy comes from the official arm website.
        let child = std::process::Command::new("arm-none-eabi-objcopy")
            .arg("--decompress-debug-sections")
            .arg(&libmpsl_original_path)
            .arg(&libmpsl_changed_path)
            .spawn()
            .expect("Could not start `arm-none-eabi-objcopy`. Is it installed and available in your path?");

        let child_result = child.wait_with_output().unwrap();
        if !child_result.status.success() {
            panic!("Something went wrong with `arm-none-eabi-objcopy`.");
        }
    }

    #[cfg(feature = "llvm-objcopy")]
    {
        // We assume the llvm-objcopy comes from the rustup llvm-tools.
        // This cannot do decompression, so we'll just strip the debug sections

        let tool_error = "Could not find `llvm-objcopy`. Is it installed? Use `rustup component add llvm-tools` to install it or select the `arm-none-eabi-objcopy` feature if you have that tool installed.";
        // It's not in our path, so we have to search for it
        let path = llvm_tools::LlvmTools::new()
            .expect(tool_error)
            .tool(&llvm_tools::exe("llvm-objcopy"))
            .expect(tool_error);

        let child = std::process::Command::new(path)
            .arg("--strip-debug")
            .arg(&libmpsl_original_path)
            .arg(&libmpsl_changed_path)
            .spawn()
            .expect(tool_error);

        let child_result = child.wait_with_output().unwrap();
        if !child_result.status.success() {
            panic!("Something went wrong with `llvm-objcopy`.");
        }
    }

    // Make sure we link against the libraries
    println!(
        "cargo:rustc-link-search={}",
        libmpsl_changed_path.parent().unwrap().display()
    );
    println!("cargo:rustc-link-lib=static=mpsl");
}
