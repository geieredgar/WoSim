#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ApiError(String);

impl From<reqwest::Error> for ApiError {
  fn from(error: reqwest::Error) -> Self {
    Self(format!("{}", error))
  }
}

#[tauri::command]
async fn release_notes() -> Result<Vec<serde_json::Value>, ApiError> {
  let mut notes = Vec::new();
  for line in reqwest::get("https://hub.wosim.net/content/releases/index.txt")
    .await?
    .text()
    .await?
    .lines()
  {
    notes.push(
      reqwest::get(format!("https://hub.wosim.net/content/releases/{}", line))
        .await?
        .json::<serde_json::Value>()
        .await?,
    )
  }
  Ok(notes)
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![release_notes])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
