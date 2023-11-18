// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;
use tauri::command; // Import the command macro for Tauri.
use serde::ser::Error as SerdeError; // Add this to bring the `Error` trait into scope.

#[derive(Debug, Serialize)]
struct ModelList {
    models: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Network request failed: {0}")]
    Network(reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(serde_json::Error),
    #[error("Command execution failed: {0}")]
    CommandError(String),
}

// Implement conversion from `ApiError` to `tauri::InvokeError`.
impl From<ApiError> for tauri::InvokeError {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Network(e) => tauri::InvokeError::from(e.to_string()), // Convert to string
            ApiError::ParseError(e) => tauri::InvokeError::from(e.to_string()), // Convert to string
            ApiError::CommandError(e) => tauri::InvokeError::from(e),
        }
    }
}

/// Greet the user with a personalized message
#[command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Asks Ollama API with a question and model, expecting a string response
#[command]
async fn askollama(question: String, models: String) -> Result<String, ApiError> {
    let url = "http://localhost:11434/api/generate";
    let client = Client::new();
    let res = client.post(url)
        .json(&json!({
            "model": models,
            "prompt": question,
            "stream": false
        }))
        .send()
        .await
        .map_err(ApiError::Network)?
        .text()
        .await
        .map_err(ApiError::Network)?;

    let final_response = res.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|val| val.get("response")?.as_str().map(ToString::to_string))
        .collect::<Vec<String>>()
        .join("");

    Ok(final_response)
}

/// Retrieves the list of models from Ollama API
#[command]
async fn get_ollama_models() -> Result<ModelList, ApiError> {

    let url = "http://localhost:11434/api/tags";
    let client = Client::new();
    let res = client.get(url)
        .send()
        .await
        .map_err(ApiError::Network)?
        .text()
        .await
        .map_err(ApiError::Network)?;

    let json_value = serde_json::from_str::<serde_json::Value>(&res)
        .map_err(ApiError::ParseError)?;
    
    let models_list = json_value["models"]
        .as_array()
        .ok_or(ApiError::ParseError(
            serde_json::error::Error::custom("ollama API responded with incorrect format")
        ))?;

    let mut models = vec![];
    for model in models_list {
        models.push(model["name"]
            .as_str()
            .ok_or(ApiError::ParseError(
                serde_json::error::Error::custom("ollama API responded with incorrect format")
            ))?
        );
    }
    Ok(ModelList { models:  models.iter().map(|&s| String::from(s)).collect() })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, askollama, get_ollama_models])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
