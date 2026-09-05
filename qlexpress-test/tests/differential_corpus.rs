//! 通过公开验收库执行完整共享差分语料的 Rust 一侧。

use std::fs;
use std::path::Path;

#[test]
fn rust_side_executes_every_shared_differential_case() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let corpus = repository.join("verification/corpus/differential.jsonl");
    let output = std::env::temp_dir().join(format!(
        "qlexpress-rust-differential-{}.jsonl",
        std::process::id()
    ));

    qlexpress_verification::differential::run(&corpus, &output)
        .expect("execute complete Rust differential corpus");
    let records = fs::read_to_string(&output)
        .expect("read Rust differential output")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    fs::remove_file(&output).expect("remove owned temporary output");
    // 293 = 295 - 2 条移入 intentional-java-divergences.jsonl 的
    // 有意偏离用例（delegate close_global no-op / operator precedence None）
    assert_eq!(records, 293, "every shared differential case must run");
}
