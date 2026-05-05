//! Handler module manifest. 28 sub-modules cover the entire `#[tauri::command]`
//! surface (Round 4 P1-1: 28 handler list locked).

pub mod auth;
pub mod backups;
pub(crate) mod common;
pub mod config;
pub mod copilot;
pub mod deeplink;
pub mod env;
pub mod failover;
pub mod global_proxy;
pub mod health;
pub mod hermes;
pub mod mcp;
pub mod model_fetch;
pub mod model_test;
pub mod omo;
pub mod openclaw;
pub mod parity;
pub mod prompts;
pub mod providers;
pub mod proxy;
pub mod sessions;
pub mod settings;
pub mod skills;
pub mod subscription;
pub mod system;
pub mod universal;
pub mod usage;
pub mod vscode;
pub mod webdav;
pub mod workspace;
