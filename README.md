# otel util

## 機能
- tokio::test部分を、マクロ一行で計装（otel trace を、指定した endpoint に送信可能）
- otel trace のコンテキスト取得
- otel の初期化処理をマニュアルで実施可能

## コード例
以下のように、tokio::testの代わりに、#[use_otel_at_test]で計装実施可能。endpoint なしだと、デフォルト：endpoint="http://localhost:4317"が設定されている。

```rust
use anyhow::{anyhow, Result};
use otel_util::*;

#[tracing::instrument(err)]
fn sample_add(a: u64, b: u64) -> Result<u64> {
    Ok(a + b)
}

#[use_otel_at_test(endpoint="http://localhost:4317")]
async fn succeed_otel_test() {
    // given
    let a = 10;
    let b = 20;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let c = sample_add(a, b).unwrap_or(0);

    // then
    assert_eq!(a + b, c);
}

```
