//! Handler module manifest. The sub-modules cover the entire `#[tauri::command]`
//! surface. (The empty Layer-2 `model_fetch` stub router was removed — audit L2;
//! real model fetch lives in `config.rs` at `/api/config/fetch-models-for-config`.)

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
pub mod model_test;
pub mod omo;
pub mod openclaw;
pub mod parity;
pub mod prompts;
pub mod providers;
pub mod proxy;
pub mod s3;
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
