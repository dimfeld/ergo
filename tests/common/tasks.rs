use ergo::tasks::{
    actions::handlers::{ActionDescription, ActionPayload},
    handlers::{NewTaskResult, TaskDescription, TaskInput, TaskResult},
    inputs::{handlers::InputPayload, Input},
};

use super::TestClient;
use reqwest::{Response, Result};

impl TestClient {
    pub async fn new_task(&self, task: &TaskInput) -> Result<NewTaskResult> {
        self.post("tasks")
            .json(task)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn update_task(&self, id: &str, task: &TaskInput) -> Result<Response> {
        let url = format!("tasks/{}", id);

        self.post(url).json(task).send().await?.error_for_status()
    }

    pub async fn list_tasks(&self) -> Result<Vec<TaskDescription>> {
        self.get("tasks")
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn get_task(&self, task_id: &str) -> Result<TaskResult> {
        let url = format!("tasks/{}", task_id);
        self.get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn delete_task(&self, task_id: &str) -> Result<Response> {
        let url = format!("tasks/{}", task_id);
        self.delete(url).send().await?.error_for_status()
    }

    pub async fn list_inputs(&self) -> Result<Vec<Input>> {
        self.get("inputs")
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn new_input(&self, input: &InputPayload) -> Result<Input> {
        self.post("inputs")
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn update_input(&self, input_id: i64, input: &InputPayload) -> Result<Input> {
        let url = format!("inputs/{}", input_id);
        self.put(url)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn delete_input(&self, input_id: i64) -> Result<Response> {
        let url = format!("inputs/{}", input_id);
        self.delete(url).send().await?.error_for_status()
    }

    pub async fn list_actions(&self) -> Result<Vec<ActionDescription>> {
        self.get("actions")
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn new_action(&self, action: &ActionPayload) -> Result<ActionDescription> {
        self.post("actions")
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn update_action(&self, action_id: i64, action: &ActionPayload) -> Result<Response> {
        let url = format!("actions/{}", action_id);
        self.put(url).send().await?.error_for_status()
    }

    pub async fn delete_action(&self, action_id: i64) -> Result<Response> {
        let url = format!("actions/{}", action_id);
        self.delete(url).send().await?.error_for_status()
    }
}
