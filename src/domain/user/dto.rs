use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
pub struct UserDto {
    #[serde(default)]
    pub id: String,
    pub username: String,
    pub password: String,
}

impl UserDto {
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Invalid user JSON: {}", e))
    }
}

#[derive(Deserialize, Serialize)]
pub struct UpdateUserDto {
    pub password: String,
}

impl UpdateUserDto {
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Invalid user JSON: {}", e))
    }
}
