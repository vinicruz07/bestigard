use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("windows-gnu") {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        // Procura por libwinapi_imm32.a no cache do Cargo para fornecer libimm32.a ao linker MinGW
        if let Ok(cargo_home) = env::var("CARGO_HOME").or_else(|_| {
            env::var("USERPROFILE").map(|p| format!("{}/.cargo", p))
        }) {
            let registry_src = PathBuf::from(cargo_home).join("registry").join("src");
            if let Ok(entries) = fs::read_dir(&registry_src) {
                for entry in entries.flatten() {
                    let winapi_lib = entry
                        .path()
                        .join("winapi-x86_64-pc-windows-gnu-0.4.0")
                        .join("lib")
                        .join("libwinapi_imm32.a");

                    if winapi_lib.exists() {
                        let dest = out_dir.join("libimm32.a");
                        let _ = fs::copy(&winapi_lib, &dest);
                        println!("cargo:rustc-link-search=native={}", out_dir.display());
                        break;
                    }
                }
            }
        }
    }
}
