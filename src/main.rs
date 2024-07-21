fn main() {
    println!("Hello, world!");
}

use anyhow::{anyhow, Result};
use otel_test::*;

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
    Err(anyhow!("some error at sample_add_panic"))
}

#[use_otel_at_test]
async fn failed_otel_test() {
    // given
    let a = 10;
    let b = 20;

    // when
    let c = sample_add_err(a, b).unwrap_or(0);

    // then
    // todo: assert_eq とかで、死ぬと、jaeger に trace を投げる前に死ぬ。
    // todo: panic で死ぬと、jaeger に trace を投げる前に死ぬ。
    assert_eq!(a + b, c);
}

#[use_otel_at_test]
async fn panic_otel_test() {
    // given
    let a = 10;
    let b = 20;

    // when
    let c = sample_add_panic(a, b).unwrap_or(0);

    // then
    // todo: assert_eq とかで、死ぬと、jaeger に trace を投げる前に死ぬ。
    // todo: panic で死ぬと、jaeger に trace を投げる前に死ぬ。
    assert_eq!(a + b, c);
}

#[use_otel_at_test]
async fn succeed_otel_test() {
    // given
    let a = 10;
    let b = 20;

    // when
    let c = sample_add(a, b).unwrap_or(0);

    // then
    assert_eq!(a + b, c);
}
