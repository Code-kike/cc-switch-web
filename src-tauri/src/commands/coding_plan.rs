use crate::services::subscription::SubscriptionQuota;

#[tauri::command]
pub async fn get_coding_plan_quota(
    base_url: String,
    api_key: String,
    // 智谱团队版（zhipu_team）靠显式标识路由（base_url 与个人版相同无法区分）。
    coding_plan_provider: Option<String>,
    team_organization_id: Option<String>,
    team_project_id: Option<String>,
) -> Result<SubscriptionQuota, String> {
    crate::services::coding_plan::get_coding_plan_quota(
        &base_url,
        &api_key,
        coding_plan_provider.as_deref(),
        team_organization_id.as_deref(),
        team_project_id.as_deref(),
    )
    .await
}
