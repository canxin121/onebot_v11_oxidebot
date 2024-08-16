pub mod bot;
pub mod event;
pub mod segment;
pub use bot::ws::OnebotV11WsBot;
pub use bot::ws_reverse::OnebotV11ReverseWsBot;

const PLATFORM: &'static str = "onebot_v11";
