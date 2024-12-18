use anyhow::anyhow;
use anyhow::Result;
use otel_util::{tracing, use_otel_at_test};

#[tracing::instrument(err)]
fn sample_add(a: u64, b: u64) -> Result<u64> {
    Ok(a + b)
}

#[tracing::instrument(err)]
fn sample_add_err(a: u64, b: u64) -> Result<u64> {
    Err(anyhow!("some error at sample_add_err"))
}

#[tracing::instrument(err)]
fn sample_add_panic(a: u64, b: u64) -> Result<u64> {
    panic!("some panic at sample_add_panic");
}

// DONE: assert_eq とかで、死ぬと、jaeger に trace を投げる前に死ぬ。
// DONE: panic で死ぬと、jaeger に trace を投げる前に死ぬ。
// DONE: 非同期タスクで死ぬ
// DONE: cargo test -- --test-threads=1, cargo test で死なないようにする。
// DONE: should_panic マクロの付与で、panic で test が通らない。
// TODO: cargo test -- --test-threads=1, cargo test でも全ての test がtrace 送信ができる。
#[should_panic]
#[use_otel_at_test]
async fn failed_otel_test() {
    // given
    let a = 10;
    let b = 20;

    // when
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let c = sample_add_err(a, b).unwrap_or(0);

    // then
    assert_eq!(a + b, c);
}

// DONE: Return type は、型がない場合にのみ、対応している。
// DONE: use_otel_at_test だけで、Error 型変える場合に対応したい。
#[should_panic]
#[use_otel_at_test]
async fn error_otel_test() -> anyhow::Result<()> {
    // given
    let a = 10;
    let b = 20;

    // when
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let c = sample_add_err(a, b)?;

    // then
    assert_eq!(100, 10);
    assert_eq!(a + b, c);
    Ok::<(), anyhow::Error>(())
}

#[should_panic]
#[use_otel_at_test]
async fn panic_otel_test() {
    // given
    let a = 10;
    let b = 20;

    // when
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let c = sample_add_panic(a, b).unwrap_or(0);

    // then
    assert_eq!(a + b, c);
}

#[use_otel_at_test]
async fn succeed_otel_test() {
    // given
    let a = 10;
    let b = 20;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let c = sample_add(a, b).unwrap_or(0);

    // then
    assert_eq!(a + b, c);
}
