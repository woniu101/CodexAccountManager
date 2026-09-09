use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub limit_id: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub window_duration_mins: u64,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAccount {
    pub id: String,
    pub email: String,
    pub alias: Option<String>,
    pub plan_type: Option<String>,
    pub is_active: bool,
    pub credential_state: String,
    pub five_hour: Option<QuotaWindow>,
    pub weekly: Option<QuotaWindow>,
    pub extra_limits: Vec<QuotaWindow>,
    pub last_updated_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardState {
    pub accounts: Vec<ManagedAccount>,
    pub codex_running: bool,
    pub refreshed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum LoginProgress {
    Idle,
    Waiting { auth_url: String },
    Duplicate { account: Box<ManagedAccount> },
    Completed { account: Box<ManagedAccount> },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPlacement {
    pub horizontal: String,
    pub vertical: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub refresh_interval_minutes: u64,
    pub launch_at_login: bool,
    pub remember_position: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            refresh_interval_minutes: 5,
            launch_at_login: false,
            remember_position: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LoginProgress;

    #[test]
    fn login_url_is_serialized_for_the_typescript_client() {
        let value = serde_json::to_value(LoginProgress::Waiting {
            auth_url: "https://example.test/login".to_string(),
        })
        .expect("serialize login progress");
        assert_eq!(value["status"], "waiting");
        assert_eq!(value["authUrl"], "https://example.test/login");
        assert!(value.get("auth_url").is_none());
    }
}
