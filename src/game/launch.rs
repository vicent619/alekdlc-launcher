use std::os::windows::process::CommandExt;
use std::process::{Child, Command};

use crate::game::command::LaunchSpec;
use crate::net::error::ApiError;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct GameProcess {
    child: Child,
}

impl GameProcess {
    pub fn spawn(spec: &LaunchSpec) -> Result<Self, ApiError> {
        let mut command = Command::new(&spec.bizuro);
        command.arg(&spec.javaw);
        command.args(&spec.args);
        command.current_dir(&spec.working_dir);
        command.env(crate::secret!("B_O"), &spec.bo_token);
        command.creation_flags(CREATE_NO_WINDOW);
        let child = command.spawn().map_err(|_| ApiError::new("LAUNCH_FAILED", 0))?;
        Ok(Self { child })
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}
