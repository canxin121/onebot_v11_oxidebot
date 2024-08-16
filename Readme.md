# OnebotV11 + Napcat/llonebot extension Bot for oxidebot

# Usage
```
cargo add onebot_v11_oxidebot
```

ReverseWsBot Example
```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let manager = oxidebot::OxideBotManager::new()
        .bot(onebot_v11_oxidebot::OnebotV11ReverseWsBot::new(Default::default()).await)
        .await;
    manager.run_block().await;
}
```