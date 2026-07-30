//! Handler module manifest. The sub-modules cover the entire `#[tauri::command]`
//! surface. (The empty Layer-2 stub routers `model_fetch`, `copilot`, `vscode`,
//! `model_test`, and `universal` were removed — audit L2; their commands are
//! `unsupported` in `web-commands.ts`, and real model fetch lives in `config.rs`
//! at `/api/config/fetch-models-for-config`.)

pub mod auth;
pub mod backups;
pub(crate) mod common;
pub mod config;
pub mod deeplink;
pub mod env;
pub mod failover;
pub mod global_proxy;
pub mod health;
pub mod hermes;
pub mod mcp;
pub mod omo;
pub mod openclaw;
pub mod parity;
pub mod profiles;
pub mod prompts;
pub mod providers;
pub mod proxy;
pub mod s3;
pub mod sessions;
pub mod settings;
pub mod skills;
pub mod subscription;
pub mod system;
pub mod usage;
pub mod webdav;
pub mod workspace;
