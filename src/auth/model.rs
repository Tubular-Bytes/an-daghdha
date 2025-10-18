#[derive(serde::Deserialize, serde::Serialize)]
pub struct AuthRequest {
    pub user: String,
    pub password: String,
}
