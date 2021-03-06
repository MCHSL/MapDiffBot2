use crate::github_types::*;
use anyhow::{Context, Result};
use octocrab::models::InstallationId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CheckRun {
    id: u64,
    installation_id: InstallationId,
    head_sha: String,
    repo: String,
}

impl CheckRun {
    pub async fn create<I: Into<InstallationId>>(
        full_repo: &str,
        head_sha: &str,
        inst_id: I,
        name: Option<&str>,
    ) -> Result<Self> {
        let inst_id = inst_id.into();
        let result: RawCheckRun = octocrab::instance()
            .installation(inst_id)
            .post(
                format!("/repos/{full_repo}/check-runs"),
                Some(&CreateCheckRun {
                    name: name.unwrap_or("MapDiffBot2").to_string(),
                    head_sha: head_sha.to_string(),
                }),
            )
            .await
            .context("Submitting check")?;

        Ok(Self {
            id: result.id,
            installation_id: inst_id,
            head_sha: head_sha.to_string(),
            repo: full_repo.to_owned(),
        })
    }

    /// Creates a new check run for the same PR
    pub async fn duplicate(&self, name: &str) -> Result<Self> {
        Self::create(&self.repo, &self.head_sha, self.installation_id, Some(name)).await
    }

    pub async fn rename(&self, name: &str) -> Result<()> {
        self.update(UpdateCheckRunBuilder::default().name(name.to_owned()))
            .await
            .context("Renaming check run")
    }

    pub async fn mark_queued(&self) -> Result<()> {
        self.update(
            UpdateCheckRunBuilder::default()
                .status("queued")
                .started_at(chrono::Utc::now().to_rfc3339()),
        )
        .await
        .context("Marking check run as queued")
    }

    pub async fn mark_started(&self) -> Result<()> {
        self.update(
            UpdateCheckRunBuilder::default()
                .status("in_progress")
                .started_at(chrono::Utc::now().to_rfc3339()),
        )
        .await
        .context("Marking check run as in progress")
    }

    pub async fn mark_failed(&self, stack_trace: &str) -> Result<()> {
        let summary = format!(
            include_str!("error_template.txt"),
            stack_trace = stack_trace
        );

        self.update(
            UpdateCheckRunBuilder::default()
                .status("completed")
                .conclusion("failure")
                .completed_at(chrono::Utc::now().to_rfc3339())
                .output(Output {
                    title: "Error handling job".to_owned(),
                    summary,
                    text: "".to_owned(),
                }),
        )
        .await
        .context("Marking check as failure")
    }

    pub async fn mark_succeeded(&self, output: Output) -> Result<()> {
        self.update(
            UpdateCheckRunBuilder::default()
                .conclusion("success")
                .completed_at(chrono::Utc::now().to_rfc3339())
                .output(output),
        )
        .await
        .context("Marking check as success")
    }

    pub async fn mark_skipped(&self, output: Output) -> Result<()> {
        self.update(
            UpdateCheckRunBuilder::default()
                .conclusion("skipped")
                .completed_at(chrono::Utc::now().to_rfc3339())
                .output(output),
        )
        .await
        .context("Marking check as skipped")
    }

    pub async fn set_output(&self, output: Output) -> Result<()> {
        self.update(UpdateCheckRunBuilder::default().output(output))
            .await
            .context("Setting check run output")
    }

    async fn update(&self, builder: UpdateCheckRunBuilder) -> Result<()> {
        let update = builder.build().context("Building UpdateCheckRun")?;

        let _: Empty = octocrab::instance()
            .installation(self.installation_id)
            .patch(
                format!(
                    "/repos/{repo}/check-runs/{check_run_id}",
                    repo = self.repo,
                    check_run_id = self.id,
                ),
                Some(&update),
            )
            .await
            .context("Updating check run")?;

        Ok(())
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

pub async fn get_pull_files(
    installation: &Installation,
    pull: &PullRequest,
) -> Result<Vec<ModifiedFile>> {
    let res = octocrab::instance()
        .installation(installation.id.into())
        .get(
            &format!(
                "/repos/{repo}/pulls/{pull_number}/files",
                repo = pull.base.repo.full_name(),
                pull_number = pull.number
            ),
            None::<&()>,
        )
        .await?;

    Ok(res)
}
