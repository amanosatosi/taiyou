#![cfg(feature = "native")]
#[test]
fn optimized_paired_experiments_on_actions() {
    if !cfg!(target_os = "linux")
        || !std::env::var("RUSTUP_TOOLCHAIN")
            .unwrap_or_default()
            .starts_with("1.99.1")
    {
        return;
    }
    let Some(folder) = std::env::var_os("TAIYOU_RENDER_SAMPLE_DIR") else {
        return;
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(
        std::process::Command::new("python3")
            .arg(root.join("experiments/t017/reuse_completed.py"))
            .arg("optimized")
            .arg(folder)
            .status()
            .unwrap()
            .success()
    );
}
