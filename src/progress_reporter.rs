use std::{time::Instant, cell::Cell};

use anyhow::Result;

use crossbeam_channel::Sender;

use lsp_server::Message;
use lsp_types::{ProgressParams, WorkDoneProgress};

pub struct ProgressReporter<'a> {
    sender: &'a Sender<Message>,
    token_counter: Cell<i32>,
}

impl<'a> ProgressReporter<'a> {
    pub fn new(sender: &Sender<Message>) -> ProgressReporter {
        ProgressReporter {
            sender,
            token_counter: Cell::new(0)
        }
    }

    pub fn track<T, F: FnOnce() -> Result<T>>(&self, title: impl AsRef<str>, f: F) -> Result<T> {
        let token = self.send_progress_begin(format!("Starting {}", title.as_ref()), "", 0)?;
        let start = Instant::now();

        let result = f()?;

        let duration = start.elapsed();

        self.send_progress_end(token, format!("{} finished in {:?}", title.as_ref(), duration))?;

        Ok(result)
    }

    pub fn send_progress_begin(
        &self,
        title: impl AsRef<str>,
        message: impl AsRef<str>,
        percentage: u32,
    ) -> Result<i32> {
        let work_done_progress_begin = lsp_types::WorkDoneProgressBegin {
            title: title.as_ref().to_string(),
            cancellable: None,
            message: Some(message.as_ref().to_string()),
            percentage: Some(percentage),
        };
        let work_done_progress = WorkDoneProgress::Begin(work_done_progress_begin);

        let token = self.token_counter.get() + 1;
        self.token_counter.set(token);
        self.send_progress(work_done_progress, token)?;

        Ok(token)
    }

    pub fn send_progress_report(&self, message: impl AsRef<str>, percentage: u32) -> Result<()> {
        let work_done_progress_report = lsp_types::WorkDoneProgressReport {
            cancellable: None,
            message: Some(message.as_ref().to_string()),
            percentage: Some(percentage),
        };
        let work_done_progress = lsp_types::WorkDoneProgress::Report(work_done_progress_report);

        let token = self.token_counter.get() + 1;
        self.token_counter.set(token);
        self.send_progress(work_done_progress, token)?;

        Ok(())
    }

    pub fn send_progress_end(&self, token: i32, message: impl AsRef<str>) -> Result<()> {
        let work_done_progress_end = lsp_types::WorkDoneProgressEnd {
            message: Some(message.as_ref().to_string()),
        };
        let work_done_progress = lsp_types::WorkDoneProgress::End(work_done_progress_end);

        self.send_progress(work_done_progress, token)?;

        Ok(())
    }

    fn send_progress(&self, work_done_progress: WorkDoneProgress, token: i32) -> Result<()> {
        let value = lsp_types::ProgressParamsValue::WorkDone(work_done_progress);

        let token = lsp_types::NumberOrString::Number(token);
        let progress_params = ProgressParams { token, value };

        let result = serde_json::to_value(progress_params)?;
        let not = lsp_server::Notification::new("$/progress".to_string(), result);

        self.sender.send(Message::Notification(not))?;

        Ok(())
    }
}
