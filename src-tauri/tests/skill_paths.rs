use cc_switch_lib::{AppType, SkillService};

#[path = "support.rs"]
mod support;
use support::{ensure_test_home, reset_test_fs, test_mutex};

#[test]
fn get_app_skills_dir_respects_test_home_override() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let path = SkillService::get_app_skills_dir(&AppType::Claude).expect("resolve skills dir");

    assert_eq!(path, home.join(".claude").join("skills"));
    assert!(
        path.starts_with(home),
        "skills dir {path:?} should live under test home {home:?}",
    );
}
