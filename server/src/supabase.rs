use crate::error::AppError;
use crate::models::{JobRecord, JobUpdate};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use uuid::Uuid;

#[derive(Clone)]
pub struct SupabaseClient {
    base_url: String,
    client: Client,
}

impl SupabaseClient {
    pub fn new(base_url: String, service_key: String) -> Result<Self, AppError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "apikey",
            HeaderValue::from_str(&service_key).map_err(|err| AppError::Config(err.to_string()))?,
        );
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {service_key}"))
                .map_err(|err| AppError::Config(err.to_string()))?,
        );
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|err| AppError::Config(err.to_string()))?;

        Ok(Self { base_url, client })
    }

    pub async fn insert_job(&self, job: &JobRecord) -> Result<JobRecord, AppError> {
        let url = format!("{}/jobs", self.base_url);
        let response = self
            .client
            .post(url)
            .header("Prefer", "return=representation")
            .json(&job)
            .send()
            .await?;
        let jobs: Vec<JobRecord> = parse_response(response).await?;
        jobs.into_iter()
            .next()
            .ok_or_else(|| AppError::Supabase("missing job response".to_string()))
    }

    pub async fn update_job(&self, job_id: Uuid, update: &JobUpdate) -> Result<JobRecord, AppError> {
        let url = format!("{}/jobs?id=eq.{}", self.base_url, job_id);
        let response = self
            .client
            .patch(url)
            .header("Prefer", "return=representation")
            .json(update)
            .send()
            .await?;
        let jobs: Vec<JobRecord> = parse_response(response).await?;
        jobs.into_iter()
            .next()
            .ok_or_else(|| AppError::Supabase("missing job response".to_string()))
    }

    pub async fn list_jobs(&self) -> Result<Vec<JobRecord>, AppError> {
        let url = format!("{}/jobs?select=*&order=created_at.desc", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_response(response).await
    }

    pub async fn get_job(&self, job_id: Uuid) -> Result<Option<JobRecord>, AppError> {
        let url = format!("{}/jobs?id=eq.{}&select=*", self.base_url, job_id);
        let response = self.client.get(url).send().await?;
        let jobs: Vec<JobRecord> = parse_response(response).await?;
        Ok(jobs.into_iter().next())
    }
}

async fn parse_response<T: DeserializeOwned>(response: Response) -> Result<T, AppError> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Supabase(format!(
            "supabase status {status}: {body}"
        )));
    }
    Ok(response.json::<T>().await?)
}
