use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const SOURCE_REPOSITORY: &str = "https://github.com/olbauday/FPL-Core-Insights";
const SOURCE_BASE_URL: &str =
    "https://raw.githubusercontent.com/olbauday/FPL-Core-Insights/main/data/2026-2027";
const DATA_SOURCE_ERROR: &str = "be.error.dataSource.updateFailed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FplDataSourceStatus {
    pub repository: String,
    pub cached: bool,
    pub updated_at: Option<String>,
    pub files: Vec<String>,
}

fn source_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("data-sources").join("fpl-core-insights"))
        .map_err(|_| DATA_SOURCE_ERROR.to_string())
}

#[tauri::command]
pub fn get_fpl_data_source_status(app: AppHandle) -> Result<FplDataSourceStatus, String> {
    let dir = source_dir(&app)?;
    let files = ["players.csv", "teams.csv"]
        .into_iter()
        .filter(|name| dir.join(name).is_file())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let updated_at = std::fs::read_to_string(dir.join("updated_at.txt")).ok();

    Ok(FplDataSourceStatus {
        repository: SOURCE_REPOSITORY.to_string(),
        cached: files.len() == 2,
        updated_at,
        files,
    })
}

/// Refresh the Premier League baseline from the publisher's repository. The
/// cache is separate from career saves, so existing careers remain pinned to
/// their creation snapshot until the import pipeline deliberately uses it.
#[tauri::command]
pub async fn update_fpl_data_source(app: AppHandle) -> Result<FplDataSourceStatus, String> {
    tauri::async_runtime::spawn_blocking(move || update_fpl_data_source_sync(&app))
        .await
        .map_err(|_| DATA_SOURCE_ERROR.to_string())?
}

fn update_fpl_data_source_sync(app: &AppHandle) -> Result<FplDataSourceStatus, String> {
    let dir = source_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|_| DATA_SOURCE_ERROR.to_string())?;

    for file_name in ["players.csv", "teams.csv"] {
        let response = reqwest::blocking::get(format!("{SOURCE_BASE_URL}/{file_name}"))
            .map_err(|_| DATA_SOURCE_ERROR.to_string())?
            .error_for_status()
            .map_err(|_| DATA_SOURCE_ERROR.to_string())?;
        let bytes = response.bytes().map_err(|_| DATA_SOURCE_ERROR.to_string())?;
        if bytes.is_empty() || bytes.len() > 50 * 1024 * 1024 {
            return Err(DATA_SOURCE_ERROR.to_string());
        }
        std::fs::write(dir.join(file_name), bytes).map_err(|_| DATA_SOURCE_ERROR.to_string())?;
    }

    let updated_at = chrono::Utc::now().to_rfc3339();
    std::fs::write(dir.join("updated_at.txt"), &updated_at)
        .map_err(|_| DATA_SOURCE_ERROR.to_string())?;
    get_fpl_data_source_status(app.clone())
}
