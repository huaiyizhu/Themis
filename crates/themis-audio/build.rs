fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    println!("cargo:rerun-if-changed=native/macos/themis_tap.m");
    println!("cargo:rerun-if-changed=native/macos/themis_tap.h");

    let deployment_target = "14.2";
    println!("cargo:rustc-link-arg=-Wl,-weak_reference_mismatches,weak");
    std::env::set_var("MACOSX_DEPLOYMENT_TARGET", deployment_target);

    cc::Build::new()
        .file("native/macos/themis_tap.m")
        .flag("-fobjc-arc")
        .flag_if_supported("-Wno-unused-parameter")
        .flag(&format!("-mmacosx-version-min={deployment_target}"))
        .include("native/macos")
        .compile("themis_tap");

    println!("cargo:rustc-link-lib=framework=CoreAudio");
    println!("cargo:rustc-link-lib=framework=AudioToolbox");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
