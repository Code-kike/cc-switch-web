//! Command manifest generator (Layer 0 / Task 0).
//!
//! Scans `src-tauri/src/` for `#[tauri::command]` attributes and the
//! `tauri::generate_handler![...]` macro registration in `lib.rs`,
//! and emits `commands.manifest.json` mapping each command to:
//!   - its source file/line
//!   - the Layer 3 task that owns its Web handler
//!   - the proposed HTTP method and path
//!   - whether the Web mode supports it
//!
//! Run from project root via `bash scripts/gen-command-manifest.sh`.
//!
//! This binary is feature-gated to compile without `desktop`; it depends on
//! `syn`, `walkdir`, `serde_json` only. See `Cargo.toml` `[[bin]]` entry.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use syn::{visit::Visit, ItemFn, Macro};
use walkdir::WalkDir;
#[derive(Debug, Serialize, Clone)]
struct CommandEntry {
    #[serde(rename = "fn")]
    fn_name: String,
    file: String,
    line: usize,
    owner: String,
    web_handler: String,
    method: String,
    path: String,
    status: String,
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let check_mode = args.iter().any(|a| a == "--check");
    let output_path = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("commands.manifest.json"));

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // src-tauri/src/ scan root: when run from src-tauri, that's `src/`; when
    // run from project root via the shell wrapper, it's `src-tauri/src/`.
    let scan_root = if manifest_dir.join("src-tauri/src").is_dir() {
        manifest_dir.join("src-tauri/src")
    } else if manifest_dir.join("src").is_dir() {
        manifest_dir.join("src")
    } else {
        eprintln!(
            "Could not locate src directory under {}",
            manifest_dir.display()
        );
        std::process::exit(1);
    };

    // Project root is the directory containing `src-tauri/`.
    let project_root = if manifest_dir.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
        manifest_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| manifest_dir.clone())
    } else {
        manifest_dir.clone()
    };

    let owner_map = build_owner_map();

    let mut commands: BTreeMap<String, CommandEntry> = BTreeMap::new();

    for entry in WalkDir::new(&scan_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(&scan_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let file = match syn::parse_file(&source) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let mut visitor = CommandVisitor {
            commands: &mut commands,
            file_rel: &rel,
            source: &source,
            owner_map: &owner_map,
        };
        visitor.visit_file(&file);
    }

    // Pull registered commands from generate_handler! to flag any orphans.
    let lib_rs = scan_root.join("lib.rs");
    let registered = if lib_rs.exists() {
        extract_generate_handler_entries(&lib_rs).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Mark any registered command not found in commands map as orphan stub.
    for r in &registered {
        commands.entry(r.clone()).or_insert_with(|| CommandEntry {
            fn_name: r.clone(),
            file: "<orphan-in-generate_handler>".into(),
            line: 0,
            owner: "Task ?".into(),
            web_handler: classify_handler(r, &owner_map).0,
            method: classify_handler(r, &owner_map).1,
            path: classify_handler(r, &owner_map).2,
            status: "pending".into(),
        });
    }

    let mut manifest: Vec<CommandEntry> = commands.into_values().collect();
    manifest.sort_by(|a, b| a.fn_name.cmp(&b.fn_name));

    let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
    let target = if output_path.is_absolute() {
        output_path
    } else {
        project_root.join(&output_path)
    };

    if check_mode {
        if !target.exists() {
            eprintln!(
                "Manifest missing at {}; run without --check first",
                target.display()
            );
            std::process::exit(2);
        }
        let existing = std::fs::read_to_string(&target).unwrap_or_default();
        if existing.trim() != json.trim() {
            eprintln!("Manifest out of sync with code at {}", target.display());
            std::process::exit(3);
        }
        eprintln!("Manifest up to date ({} commands).", manifest.len());
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, &json)?;
    eprintln!("Wrote {} commands to {}", manifest.len(), target.display());
    Ok(())
}

struct CommandVisitor<'a> {
    commands: &'a mut BTreeMap<String, CommandEntry>,
    file_rel: &'a str,
    source: &'a str,
    owner_map: &'a OwnerMap,
}

impl<'a> Visit<'a> for CommandVisitor<'a> {
    fn visit_item_fn(&mut self, node: &'a ItemFn) {
        let has_command_attr = node.attrs.iter().any(is_tauri_command_attr);
        if !has_command_attr {
            syn::visit::visit_item_fn(self, node);
            return;
        }

        let fn_name = node.sig.ident.to_string();
        let line = locate_fn_line(self.source, &fn_name);
        let (web_handler, method, path) = classify_handler(&fn_name, self.owner_map);
        let owner = owner_for_handler(&web_handler);
        let status = status_for(&fn_name, &web_handler);

        self.commands.insert(
            fn_name.clone(),
            CommandEntry {
                fn_name,
                file: self.file_rel.to_string(),
                line,
                owner,
                web_handler,
                method,
                path,
                status,
            },
        );

        syn::visit::visit_item_fn(self, node);
    }

    fn visit_macro(&mut self, mac: &'a Macro) {
        // Could parse generate_handler! tokens here; we use a separate pass.
        syn::visit::visit_macro(self, mac);
    }
}

fn is_tauri_command_attr(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    if path.is_ident("command") {
        return true;
    }
    let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    segs == vec!["tauri".to_string(), "command".to_string()]
}

fn locate_fn_line(source: &str, fn_name: &str) -> usize {
    // Locate the line that declares this function. Handles `pub fn`,
    // `pub(crate) fn`, `pub async fn`, `async fn`, and bare `fn`.
    let bare = format!("fn {}", fn_name);
    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        // Strip leading `pub`, `pub(...)`, `async`, `unsafe`, and whitespace.
        let mut rest = trimmed;
        loop {
            if rest.starts_with("pub(") {
                if let Some(end) = rest.find(')') {
                    rest = rest[end + 1..].trim_start();
                    continue;
                }
            }
            if let Some(stripped) = rest
                .strip_prefix("pub ")
                .or_else(|| rest.strip_prefix("async "))
                .or_else(|| rest.strip_prefix("unsafe "))
                .or_else(|| rest.strip_prefix("const "))
            {
                rest = stripped.trim_start();
                continue;
            }
            break;
        }
        if rest.starts_with(&bare) {
            // Verify the next char is `(` or `<` to avoid prefix matches.
            let after = &rest[bare.len()..];
            if after
                .chars()
                .next()
                .map_or(true, |c| c == '(' || c == '<' || c.is_whitespace())
            {
                return idx + 1;
            }
        }
    }
    1
}

fn extract_generate_handler_entries(path: &Path) -> Option<Vec<String>> {
    let source = std::fs::read_to_string(path).ok()?;
    let file = syn::parse_file(&source).ok()?;

    let mut entries: Vec<String> = Vec::new();
    let mut visitor = HandlerCollector {
        entries: &mut entries,
    };
    visitor.visit_file(&file);
    Some(entries)
}

struct HandlerCollector<'a> {
    entries: &'a mut Vec<String>,
}

impl<'a> Visit<'a> for HandlerCollector<'a> {
    fn visit_macro(&mut self, mac: &'a Macro) {
        let path_str: Vec<String> = mac
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let is_handler = matches!(path_str.as_slice(),
            [a] if a == "generate_handler"
        ) || matches!(path_str.as_slice(),
            [a, b] if a == "tauri" && b == "generate_handler"
        );
        if is_handler {
            // Tokenize and collect identifiers; macro body is a comma list of paths.
            let tokens = mac.tokens.to_string();
            for raw in tokens.split(',') {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Take the last :: segment as the function name.
                let name = trimmed
                    .split("::")
                    .last()
                    .unwrap_or(trimmed)
                    .trim()
                    .trim_end_matches(',')
                    .to_string();
                if !name.is_empty()
                    && name
                        .chars()
                        .next()
                        .map_or(false, |c| c.is_lowercase() || c == '_')
                {
                    self.entries.push(name);
                }
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

// --- Owner / handler classification -----------------------------------------

type OwnerMap = BTreeMap<&'static str, (&'static str, &'static str)>;

fn build_owner_map() -> OwnerMap {
    // keyword pattern -> (web_handler, owner Task)
    let mut m: OwnerMap = BTreeMap::new();
    m.insert("provider", ("providers", "Task 5"));
    m.insert("import_live", ("providers", "Task 5"));
    m.insert("mcp", ("mcp", "Task 5"));
    m.insert("skill", ("skills", "Task 5"));
    m.insert("prompt", ("prompts", "Task 5"));
    m.insert("setting", ("settings", "Task 5"));
    m.insert("config", ("config", "Task 5"));
    m.insert("import_export", ("config", "Task 5"));
    m.insert("backup", ("backups", "Task 7"));
    m.insert("webdav", ("webdav", "Task 7"));
    m.insert("proxy", ("proxy", "Task 6"));
    m.insert("global_proxy", ("global_proxy", "Task 6"));
    m.insert("failover", ("failover", "Task 6"));
    m.insert("usage", ("usage", "Task 6"));
    m.insert("balance", ("usage", "Task 6"));
    m.insert("coding_plan", ("usage", "Task 6"));
    m.insert("subscription", ("subscription", "Task 6"));
    m.insert("session_manager", ("sessions", "Task 7"));
    m.insert("session", ("sessions", "Task 7"));
    m.insert("hermes", ("hermes", "Task 7"));
    m.insert("openclaw", ("openclaw", "Task 7"));
    m.insert("omo", ("omo", "Task 7"));
    m.insert("workspace", ("workspace", "Task 7"));
    m.insert("model_fetch", ("model_fetch", "Task 7"));
    m.insert("model_test", ("model_test", "Task 7"));
    m.insert("stream_check", ("model_test", "Task 7"));
    m.insert("speedtest", ("model_test", "Task 7"));
    m.insert("env", ("env", "Task 7"));
    m.insert("env_check", ("env", "Task 7"));
    m.insert("deeplink", ("deeplink", "Task 7"));
    m.insert("auth", ("auth", "Task 7"));
    m.insert("copilot", ("copilot", "Task 7"));
    m.insert("vscode", ("vscode", "Task 7"));
    m.insert("plugin", ("vscode", "Task 7"));
    m.insert("lightweight", ("system", "Task 4"));
    m.insert("misc", ("system", "Task 4"));
    m.insert("sync_support", ("system", "Task 4"));
    m.insert("codex_oauth", ("subscription", "Task 6"));
    m
}

fn classify_handler(fn_name: &str, owner_map: &OwnerMap) -> (String, String, String) {
    let lower = fn_name.to_lowercase();
    for (key, (handler, _owner)) in owner_map.iter() {
        if lower.contains(key) {
            let method = guess_method(&lower);
            let path = guess_path(handler, &lower);
            return (handler.to_string(), method, path);
        }
    }
    (
        "system".to_string(),
        "POST".to_string(),
        format!("/api/system/{}", fn_name),
    )
}

fn owner_for_handler(handler: &str) -> String {
    match handler {
        "health" | "system" => "Task 4".into(),
        "providers" | "universal" | "mcp" | "prompts" | "skills" | "settings" | "config" => {
            "Task 5".into()
        }
        "proxy" | "global_proxy" | "failover" | "usage" | "subscription" => "Task 6".into(),
        _ => "Task 7".into(),
    }
}

fn guess_method(name: &str) -> String {
    if name.starts_with("get_")
        || name.starts_with("list_")
        || name.starts_with("read_")
        || name.starts_with("scan_")
        || name.starts_with("load_")
        || name.starts_with("query_")
        || name.starts_with("fetch_")
        || name.starts_with("check_")
        || name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("inspect_")
    {
        "GET".to_string()
    } else if name.starts_with("delete_")
        || name.starts_with("remove_")
        || name.starts_with("clear_")
    {
        "DELETE".to_string()
    } else if name.starts_with("update_")
        || name.starts_with("set_")
        || name.starts_with("save_")
        || name.starts_with("write_")
    {
        "PUT".to_string()
    } else {
        "POST".to_string()
    }
}

fn guess_path(handler: &str, fn_name: &str) -> String {
    format!(
        "/api/{}/{}",
        handler.replace('_', "-"),
        fn_name.replace('_', "-")
    )
}

fn status_for(fn_name: &str, handler: &str) -> String {
    let lower = fn_name.to_lowercase();
    let web_replacement_fn_names = [
        "export_config_to_file",
        "import_config_from_file",
        "import_prompt_from_file",
        "install_skills_from_zip",
        "open_file_dialog",
        "open_zip_file_dialog",
        "save_file_dialog",
    ];
    if web_replacement_fn_names.contains(&lower.as_str()) {
        return "web_replacement".into();
    }
    let unsupported_fn_names = [
        "open_app_config_folder",
        "open_config_folder",
        "open_provider_terminal",
        "open_workspace_directory",
        "pick_directory",
    ];
    if unsupported_fn_names.contains(&lower.as_str()) {
        return "not_supported_in_web".into();
    }
    let unsupported_keywords = [
        "tray",
        "window",
        "notification",
        "deeplink_register",
        "auto_launch",
        "dialog_open",
        "shell_",
        "dmg_",
        "clipboard",
        "copilot_",
        "codex_oauth",
        "vscode_",
        "plugin_install",
        "single_instance",
    ];
    if unsupported_keywords.iter().any(|k| lower.contains(k)) {
        return "not_supported_in_web".into();
    }
    if matches!(handler, "copilot" | "vscode") {
        return "not_supported_in_web".into();
    }
    "pending".into()
}
