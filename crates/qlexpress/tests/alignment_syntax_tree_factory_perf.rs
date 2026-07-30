//! 对齐 Java `com.alibaba.qlexpress4.aparser.SyntaxTreeFactoryPerfTest`。
//!
//! 使用从 Java `src/test/resources/perf` 固定迁移的原始语料，验证解析
//! 成功及相同的 1s/2s 超时边界。

use std::time::{Duration, Instant};

use qlexpress::Express4Runner;

const COMPLEX_IF_CONDITION: &str = include_str!("fixtures/perf/complex_if_condition.ql");
const LONG_ONE_LINE_FORMAT: &str = include_str!("fixtures/perf/long_one_line_format.ql");
const LONG_ONE_LINE_SIMPLE: &str = include_str!("fixtures/perf/long_one_line_simple.ql");

fn coverage_instrumented() -> bool {
    std::env::var_os("LLVM_PROFILE_FILE").is_some()
}

fn parse_within(script: &str, timeout: Duration) {
    let runner = Express4Runner::new();
    let start = Instant::now();
    runner
        .parse_to_syntax_tree(script)
        .expect("performance fixture must parse");
    let elapsed = start.elapsed();
    if !coverage_instrumented() {
        assert!(
            elapsed < timeout,
            "fixture parse exceeded {timeout:?}: {elapsed:?}"
        );
    }
}

/// Java 的 `long_one_line.ql` 与格式化 fixture 仅在字符串外空白不同。
/// 从同一固定语料派生可避免两份 10K/20K 文件发生内容漂移。
fn remove_whitespace_outside_strings(script: &str) -> String {
    let mut result = String::with_capacity(script.len());
    let mut quote = None;
    let mut escaped = false;
    for character in script.chars() {
        if let Some(active_quote) = quote {
            result.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
            result.push(character);
        } else if !character.is_whitespace() {
            result.push(character);
        }
    }
    result
}

// Java source: SyntaxTreeFactoryPerfTest#complexIfTestWithProfile
#[test]
fn complex_if_test_with_profile() {
    parse_within(COMPLEX_IF_CONDITION, Duration::from_secs(1));
}

// Java source: SyntaxTreeFactoryPerfTest#longOneLineSimpleProfile
#[test]
fn long_one_line_simple_profile() {
    parse_within(LONG_ONE_LINE_SIMPLE, Duration::from_secs(1));
}

// Java source: SyntaxTreeFactoryPerfTest#longOneLineProfile
#[test]
fn long_one_line_profile() {
    let one_line = remove_whitespace_outside_strings(LONG_ONE_LINE_FORMAT);
    // 锁定 Java 原始 long_one_line.ql（无末尾换行）：UTF-8 为
    // 11,240 字节，Java/Rust Unicode 标量字符计数为 10,640。
    assert_eq!(one_line.len(), 11_240);
    assert_eq!(one_line.chars().count(), 10_640);
    parse_within(&one_line, Duration::from_secs(1));
}

// Java source: SyntaxTreeFactoryPerfTest#longOneLineFormatProfile
#[test]
fn long_one_line_format_profile() {
    parse_within(LONG_ONE_LINE_FORMAT, Duration::from_secs(2));
}

// Java source: SyntaxTreeFactoryPerfTest#oneLineIfTest
#[test]
fn one_line_if_test() {
    let script = "Y_D_ / (if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 65 )) { 1500; } else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 80  &&  Y_D_ >= 65 )) { 1200; }  else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 80  &&  Y_D_ >= 65 )) { 1200; } else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 100  &&  Y_D_ >= 80 )) { 1000; } else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 120  &&  Y_D_ >= 100 )) { 800; } else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 150  &&  Y_D_ >= 120 )) { 600; } else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 180  &&  Y_D_ >= 150 )) { 400; } else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ < 200  &&  Y_D_ >= 180 )) { 300; } else if (( item in  [ 'S45C','SUS304' ]   &&  Y_D_ <= 230  &&  Y_D_ >= 200 )) { 200; } else if (( item in  [ '金','POM' ]   &&  Y_D_ < 65 )) { 2000; } else if (( item in  [ '金','POM' ]   &&  Y_D_ < 80  &&  Y_D_ >= 65 )) { 1500; } else if (( item in  [ '金','POM' ]   &&  Y_D_ < 100  &&  Y_D_ >= 80 )) { 1200; } else if (( item in  [ '金','POM' ]   &&  Y_D_ < 150  &&  Y_D_ >= 100 )) { 1000; } else if (( item in  [ '金','POM' ]   &&  Y_D_ < 200  &&  Y_D_ >= 150 )) { 800; } else if (( item in  [ '金','POM' ]   &&  Y_D_ <= 230  &&  Y_D_ >= 200 )) { 600; } else { null } )";
    parse_within(script, Duration::from_secs(1));
}
