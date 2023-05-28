use anyhow::Result;

use crossbeam_channel::Sender;

use lsp_server::Message;
use lsp_types::{ProgressParams, WorkDoneProgress};

pub struct ProgressReporter<'a> {
    sender: &'a Sender<Message>,
    token_counter: i32,
}

impl<'a> ProgressReporter<'a> {
    pub fn new(sender: &Sender<Message>) -> ProgressReporter {
        ProgressReporter {
            sender,
            token_counter: 0
        }
    }

    pub fn send_progress_begin(
        &mut self,
        title: &str,
        message: &str,
        percentage: u32,
    ) -> Result<i32> {
        let work_done_progress_begin = lsp_types::WorkDoneProgressBegin {
            title: title.to_string(),
            cancellable: None,
            message: Some(message.to_string()),
            percentage: Some(percentage),
        };
        let work_done_progress = WorkDoneProgress::Begin(work_done_progress_begin);

        self.token_counter += 1;
        self.send_progress(work_done_progress, self.token_counter)?;

        Ok(self.token_counter)
    }

    pub fn send_progress_report(&mut self, message: &str, percentage: u32) -> Result<()> {
        let work_done_progress_report = lsp_types::WorkDoneProgressReport {
            cancellable: None,
            message: Some(message.to_owned()),
            percentage: Some(percentage),
        };
        let work_done_progress = lsp_types::WorkDoneProgress::Report(work_done_progress_report);

        self.token_counter += 1;
        self.send_progress(work_done_progress, self.token_counter)?;

        Ok(())
    }

    pub fn send_progress_end(&mut self, token: i32, message: &str) -> Result<()> {
        let work_done_progress_end = lsp_types::WorkDoneProgressEnd {
            message: Some(message.to_string()),
        };
        let work_done_progress = lsp_types::WorkDoneProgress::End(work_done_progress_end);

        self.send_progress(work_done_progress, token)?;

        Ok(())
    }

    fn send_progress(&mut self, work_done_progress: WorkDoneProgress, token: i32) -> Result<()> {
        let value = lsp_types::ProgressParamsValue::WorkDone(work_done_progress);

        let token = lsp_types::NumberOrString::Number(token);
        let progress_params = ProgressParams { token, value };

        let result = serde_json::to_value(progress_params)?;
        let not = lsp_server::Notification::new("$/progress".to_string(), result);

        self.sender.send(Message::Notification(not))?;

        Ok(())
    }
}
