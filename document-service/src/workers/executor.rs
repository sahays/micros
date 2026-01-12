use service_core::error::AppError;
use std::path::Path;
use std::process::Output;
use std::time::Duration;
use tokio::process::Command;

#[derive(Clone)]
pub struct CommandExecutor {
    timeout: Duration,
}

impl CommandExecutor {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub async fn execute(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<Output, AppError> {
        let mut cmd = Command::new(program);
        cmd.args(args);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        tracing::debug!(
            program = %program,
            args = ?args,
            timeout_secs = %self.timeout.as_secs(),
            "Executing command"
        );

        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .map_err(|_| {
                AppError::InternalError(anyhow::anyhow!(
                    "Command timed out after {} seconds",
                    self.timeout.as_secs()
                ))
            })??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!(
                program = %program,
                args = ?args,
                stderr = %stderr,
                "Command failed"
            );
            return Err(AppError::InternalError(anyhow::anyhow!(
                "Command failed: {}",
                stderr
            )));
        }

        tracing::debug!(
            program = %program,
            output_size = output.stdout.len(),
            "Command succeeded"
        );

        Ok(output)
    }
}
