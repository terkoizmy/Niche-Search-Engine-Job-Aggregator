use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub title: String,
    pub company: String,
    pub location: String,
    pub description: String,
    pub salary_raw: String,
    pub salary_min: Option<i64>,
    pub url: String,
}
