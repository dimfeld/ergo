use ergo_api::tasks::{
    actions::handlers::{ActionDescription, ActionPayload},
    handlers::{
        InputsLogEntry, NewTaskResult, TaskDescription, TaskInput, TaskResult, TaskTriggerResponse,
    },
    inputs::{handlers::InputPayload, Input},
};
use ergo_database::object_id::{ActionId, InputId, TaskId};

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

    pub async fn put_task(&self, id: &TaskId, task: &TaskInput) -> Result<Response> {
        let url = format!("tasks/{}", id);

        self.put(url).json(task).send().await?.error_for_status()
    }

    pub async fn list_tasks(&self) -> Result<Vec<TaskDescription>> {
        self.get("tasks")
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn get_task(&self, task_id: &TaskId) -> Result<TaskResult> {
        let url = format!("tasks/{}", task_id);
        self.get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn delete_task(&self, task_id: &TaskId) -> Result<Response> {
        let url = format!("tasks/{}", task_id);
        self.delete(url).send().await?.error_for_status()
    }

    pub async fn run_task_trigger(
        &self,
        task: &str,
        trigger: &str,
        payload: serde_json::Value,
    ) -> Result<TaskTriggerResponse> {
        let url = format!("tasks/{}/trigger/{}", task, trigger);
        self.post(url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
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
            .json(input)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn put_input(&self, input_id: &InputId, input: &InputPayload) -> Result<Input> {
        let url = format!("inputs/{}", input_id);
        self.put(url)
            .json(input)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn delete_input(&self, input_id: &InputId) -> Result<Response> {
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
            .json(action)
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }

    pub async fn put_action(
        &self,
        action_id: &ActionId,
        action: &ActionPayload,
    ) -> Result<Response> {
        let url = format!("actions/{}", action_id);
        self.put(url).json(action).send().await?.error_for_status()
    }

    pub async fn delete_action(&self, action_id: &ActionId) -> Result<Response> {
        let url = format!("actions/{}", action_id);
        self.delete(url).send().await?.error_for_status()
    }

    pub async fn get_recent_logs(&self) -> Result<Vec<InputsLogEntry>> {
        self.get("logs")
            .send()
            .await?
            .error_for_status()?
            .json::<_>()
            .await
    }
}
