//! Build script for rusty_mapper
//! 
//! Sets the rpath for Syphon framework on macOS

fn main() {
    #[cfg(target_os = "macos")]
    {
        // Path to the Syphon framework
        let framework_dir = std::path::PathBuf::from("../crates/syphon/syphon-lib");
        
        if framework_dir.exists() {
            let canonical = framework_dir.canonicalize().unwrap();
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", canonical.display());
            println!("cargo:warning=Setting rpath for Syphon: {}", canonical.display());
        }
        
        println!("cargo:rerun-if-changed=build.rs");
    }
}
