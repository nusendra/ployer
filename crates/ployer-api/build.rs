fn main() {
    // Read version from the latest git tag (e.g. v0.1.0-alpha.17 → 0.1.0-alpha.17)
    let version = std::process::Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().trim_start_matches('v').to_string())
        .unwrap_or_else(|| {
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string())
        });

    println!("cargo:rustc-env=PLOYER_VERSION={}", version);
    // Re-run when HEAD or packed refs change (new tag / commit)
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/packed-refs");
}
