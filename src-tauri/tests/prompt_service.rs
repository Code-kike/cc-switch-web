use std::fs;

use cc_switch_lib::{prompt_file_path, AppType, Prompt, PromptService};

#[path = "support.rs"]
mod support;
use support::{create_test_state, reset_test_fs, test_mutex};

fn make_prompt(id: &str, content: &str, enabled: bool) -> Prompt {
    Prompt {
        id: id.to_string(),
        name: format!("Prompt {id}"),
        content: content.to_string(),
        description: None,
        enabled,
        created_at: Some(1),
        updated_at: Some(1),
    }
}

#[test]
fn upsert_disabled_prompt_does_not_touch_existing_live_file() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let state = create_test_state().expect("create test state");
    let target_path = prompt_file_path(&AppType::Claude).expect("resolve prompt path");
    let live_content = "keep existing live prompt";

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).expect("create prompt dir");
    }
    fs::write(&target_path, live_content).expect("seed live prompt file");

    PromptService::upsert_prompt(
        &state,
        AppType::Claude,
        "imported-1",
        make_prompt("imported-1", "uploaded prompt", false),
    )
    .expect("save disabled prompt");

    let saved = state
        .db
        .get_prompts(AppType::Claude.as_str())
        .expect("load prompts");
    assert_eq!(saved.len(), 1);
    assert!(
        !saved.get("imported-1").expect("saved prompt").enabled,
        "imported prompt should remain disabled",
    );
    assert_eq!(
        fs::read_to_string(&target_path).expect("read live prompt file"),
        live_content,
        "saving a disabled prompt must not clear or overwrite the current live file",
    );
}

#[test]
fn disabling_last_enabled_prompt_clears_live_file() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let state = create_test_state().expect("create test state");
    let target_path = prompt_file_path(&AppType::Claude).expect("resolve prompt path");

    PromptService::upsert_prompt(
        &state,
        AppType::Claude,
        "prompt-1",
        make_prompt("prompt-1", "enabled prompt", true),
    )
    .expect("save enabled prompt");
    assert_eq!(
        fs::read_to_string(&target_path).expect("read live prompt file"),
        "enabled prompt",
    );

    PromptService::upsert_prompt(
        &state,
        AppType::Claude,
        "prompt-1",
        make_prompt("prompt-1", "enabled prompt", false),
    )
    .expect("disable prompt");

    let saved = state
        .db
        .get_prompts(AppType::Claude.as_str())
        .expect("load prompts");
    assert!(
        !saved.get("prompt-1").expect("saved prompt").enabled,
        "prompt should be disabled in database",
    );
    assert_eq!(
        fs::read_to_string(&target_path).expect("read live prompt file"),
        "",
        "disabling the last enabled prompt should clear the live file",
    );
}
