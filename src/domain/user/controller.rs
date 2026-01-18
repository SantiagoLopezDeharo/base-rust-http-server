use std::collections::HashMap;

use crate::primitives::http::request::Request;
use crate::primitives::http::response::Response;
use crate::route;
use crate::routing::{Route, RouteParams};

use super::repo::UserRepo;
use super::service::UserService;
use uuid::Uuid;

pub struct UserController;

impl UserController {
    pub fn routes() -> Vec<Route> {
        vec![
            Route::new("GET", &["user"], vec![route!(UserController::get_all)]),
            Route::new("POST", &["user"], vec![route!(UserController::create)]),
            Route::new(
                "GET",
                &["user", ":id"],
                vec![route!(UserController::get_one)],
            ),
            Route::new(
                "PUT",
                &["user", ":id"],
                vec![route!(UserController::update)],
            ),
            Route::new(
                "DELETE",
                &["user", ":id"],
                vec![route!(UserController::delete)],
            ),
        ]
    }

    pub async fn get_all(_request: &mut Request, _params: &RouteParams) -> Response {
        // Use query_params from request
        let top = _request
            .query_params
            .get("top")
            .and_then(|v| v.parse().ok());

        let skip = _request
            .query_params
            .get("skip")
            .and_then(|v| v.parse().ok());

        let service = UserService::new(UserRepo::new());

        let mut headers = HashMap::new();

        headers.insert("Content-Type".to_string(), "application/json".to_string());

        match service.get_all_paginated(top, skip).await {
            Ok(body) => Response {
                status_code: 200,
                headers,
                body,
            },

            Err(e) => Response {
                status_code: 500,
                headers,
                body: format!("Failed to fetch users: {}", e),
            },
        }
    }

    pub async fn get_one(_request: &mut Request, params: &RouteParams) -> Response {
        let _id = params.get("id").unwrap_or("");

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // Validate UUID
        if Uuid::parse_str(_id).is_err() {
            return Response {
                status_code: 400,
                headers,
                body: format!(
                    "{{\"error\":{}}}",
                    serde_json::json!(format!(
                        "Invalid UUID for user id: '{}'. Must be a valid UUID string.",
                        _id
                    ))
                ),
            };
        }

        let service = UserService::new(UserRepo::new());
        match service.get_one(_id.to_string()).await {
            Ok(body) => Response {
                status_code: 200,
                headers,
                body,
            },
            Err(e) => Response {
                status_code: 500,
                headers,
                body: format!("{{\"error\":{}}}", serde_json::json!(e.to_string())),
            },
        }
    }

    pub async fn create(request: &mut Request, _params: &RouteParams) -> Response {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());

        let user = match super::dto::UserDto::from_json(&request.body) {
            Ok(user) => user,
            Err(err) => {
                return Response {
                    status_code: 400,
                    headers,
                    body: err,
                };
            }
        };

        let service = UserService::new(UserRepo::new());

        if let Err(e) = service.create_user(user).await {
            return Response {
                status_code: 500,
                headers,
                body: format!("Failed to create user: {}", e),
            };
        }

        Response {
            status_code: 201,
            headers,
            body: "".to_string(),
        }
    }

    pub async fn update(request: &mut Request, params: &RouteParams) -> Response {
        let _id = params.get("id").unwrap_or("").to_string();

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // Validate UUID
        if Uuid::parse_str(&_id).is_err() {
            return Response {
                status_code: 400,
                headers,
                body: format!(
                    "{{\"error\":{}}}",
                    serde_json::json!(format!(
                        "Invalid UUID for user id: '{}'. Must be a valid UUID string.",
                        _id
                    ))
                ),
            };
        }

        let user = match super::dto::UpdateUserDto::from_json(&request.body) {
            Ok(user) => user,
            Err(err) => {
                return Response {
                    status_code: 400,
                    headers,
                    body: err,
                };
            }
        };

        let service = UserService::new(UserRepo::new());

        match service.update_user(_id, user.password).await {
            Ok(_) => Response {
                status_code: 200,
                headers,
                body: "".to_string(),
            },
            Err(e) => Response {
                status_code: 500,
                headers,
                body: format!("Failed to create user: {}", e),
            },
        }
    }

    pub async fn delete(_request: &mut Request, params: &RouteParams) -> Response {
        let _id = params.get("id").unwrap_or("").to_string();
        let service = UserService::new(UserRepo::new());

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // Validate UUID
        if Uuid::parse_str(&_id).is_err() {
            return Response {
                status_code: 400,
                headers,
                body: format!(
                    "{{\"error\":{}}}",
                    serde_json::json!(format!(
                        "Invalid UUID for user id: '{}'. Must be a valid UUID string.",
                        _id
                    ))
                ),
            };
        }

        match service.delete_user(_id).await {
            Ok(_) => Response {
                status_code: 200,
                headers,
                body: "".to_string(),
            },
            Err(e) => Response {
                status_code: 500,
                headers,
                body: format!("Failed to create user: {}", e),
            },
        }
    }
}
