// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use debuggable_module::loader::Loader;

use crate::allowlist::TargetAllowList;
use crate::binary::BinaryCoverage;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

pub struct CoverageRecorder {
    allowlist: TargetAllowList,
    cmd: Command,
    loader: Arc<Loader>,
    timeout: Duration,
}

impl CoverageRecorder {
    pub fn new(mut cmd: Command) -> Self {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let allowlist = TargetAllowList::default();
        let loader = Arc::new(Loader::new());
        let timeout = Duration::from_secs(5);

        Self {
            allowlist,
            cmd,
            loader,
            timeout,
        }
    }

    pub fn allowlist(mut self, allowlist: TargetAllowList) -> Self {
        self.allowlist = allowlist;
        self
    }

    pub fn loader(mut self, loader: impl Into<Arc<Loader>>) -> Self {
        self.loader = loader.into();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[cfg(target_os = "linux")]
    pub fn record(self) -> Result<Recorded> {
        use linux::debugger::Debugger;
        use linux::LinuxRecorder;

        let loader = self.loader.clone();

        crate::timer::timed(self.timeout, move || {
            let mut recorder = LinuxRecorder::new(&loader, self.allowlist);
            let dbg = Debugger::new(&mut recorder);
            let output = dbg.run(self.cmd)?;
            let coverage = recorder.coverage;

            Ok(Recorded { coverage, output })
        })?
    }

    #[cfg(target_os = "windows")]
    pub fn record(self) -> Result<Recorded> {
        use debugger::Debugger;
        use windows::WindowsRecorder;

        let loader = self.loader.clone();

        crate::timer::timed(self.timeout, move || {
            let mut recorder = WindowsRecorder::new(&loader, self.allowlist);
            let (mut dbg, child) = Debugger::init(self.cmd, &mut recorder)?;
            dbg.run(&mut recorder)?;

            // If the debugger callbacks fail, this may return with a spurious clean exit.
            let output = child.wait_with_output()?.into();

            // Check if debugging was stopped due to a callback error.
            //
            // If so, the debugger terminated the target, and the recorded coverage and
            // output are both invalid.
            if let Some(err) = recorder.stop_error {
                return Err(err);
            }

            let coverage = recorder.coverage;

            Ok(Recorded { coverage, output })
        })?
    }
}

#[derive(Clone, Debug)]
pub struct Recorded {
    pub coverage: BinaryCoverage,
    pub output: Output,
}

#[derive(Clone, Debug, Default)]
pub struct Output {
    pub status: Option<ExitStatus>,
    pub stderr: String,
    pub stdout: String,
}

impl From<std::process::Output> for Output {
    fn from(output: std::process::Output) -> Self {
        let status = Some(output.status);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Self {
            status,
            stdout,
            stderr,
        }
    }
}
