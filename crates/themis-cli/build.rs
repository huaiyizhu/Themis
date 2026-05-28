fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let plist = std::path::Path::new("Info.plist");
    println!("cargo:rerun-if-changed=Info.plist");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::copy(plist, out_dir.join("ThemisCli-Info.plist")).expect("copy Info.plist");

    println!(
        "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}",
        out_dir.join("ThemisCli-Info.plist").display()
    );
}
