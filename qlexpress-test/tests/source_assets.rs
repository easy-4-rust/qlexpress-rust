//! 验证 Java 测试资源的不可变原样副本。

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

fn sha256(path: &Path) -> String {
    let mut digest = Sha256::new();
    digest.update(fs::read(path).expect("read immutable source asset"));
    format!("{:x}", digest.finalize())
}

#[test]
fn source_test_resources_and_manifest_exist() {
    let resources = qlexpress_test::source_test_resources();
    assert!(
        resources.is_dir(),
        "missing source resources: {}",
        resources.display()
    );

    let manifest = qlexpress_test::source_test_parity_manifest();
    assert!(
        manifest.is_file(),
        "missing parity manifest: {}",
        manifest.display()
    );
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("read source-test parity manifest"))
            .expect("parse source-test parity manifest");
    assert_eq!(document["schema"], 1);
    assert_eq!(document["acceptance_module"]["package"], "qlexpress-test");
    assert_eq!(document["acceptance_module"]["publish"], false);

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let assets = document["assets"].as_array().expect("asset ledger");
    assert_eq!(assets.len(), 236, "complete Java resource denominator");
    for asset in assets {
        let target = repository.join(asset["target"].as_str().expect("target path"));
        let expected = asset["sha256"].as_str().expect("source SHA-256");
        assert_eq!(sha256(&target), expected, "asset: {}", target.display());
        assert_eq!(asset["mode"], "COPY_EXACT");
    }

    let source_tests = document["source_tests"]
        .as_array()
        .expect("source test ledger");
    assert_eq!(
        source_tests.len(),
        223,
        "complete Java test-method denominator"
    );
    for source_test in source_tests {
        for target in source_test["targets"].as_array().expect("target list") {
            let target = target.as_str().expect("target marker");
            let (file, _) = target.split_once('#').expect("file#test marker");
            assert!(
                repository.join(file).is_file(),
                "missing mapped test: {file}"
            );
        }
    }
}
