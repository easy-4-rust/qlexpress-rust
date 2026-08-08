//! QlExpress Java 到 Rust 的整体验收测试包。
//!
//! 本包不承载生产逻辑，只统一拥有源测试资源、整套脚本回放、Java/Rust
//! 差分结果与跨 crate 生产验收证据。对应 Java 基线仓库的完整 `src/test`
//! 分母；生产 crate 内的局部测试不能替代本包。

use std::path::{Path, PathBuf};

/// 返回逐字节复制的 Java `src/test/resources` 根目录。
pub fn source_test_resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("suite")
        .join("source")
        .join("src")
        .join("test")
        .join("resources")
}

/// 返回验收包内的源测试清单。
pub fn source_test_parity_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("source-test-parity.json")
}
