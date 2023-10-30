use clap::Args;

use crate::{execute_command, pid_file_path, process_alive, read_pid, root_fs_path, write_pid};

#[derive(Debug, Args)]
pub struct ExecArgs {
    pub container: String,
    pub command: String,
    pub command_args: Vec<String>,
    #[clap(long, short, default_value_t = false)]
    pub force: bool,
}

impl ExecArgs {
    pub fn run(self) -> anyhow::Result<()> {
        // Lock this container
        let pid_file_path = pid_file_path(&self.container);
        let pid = read_pid(&pid_file_path);
        if let Some(pid) = pid {
            if !self.force && process_alive(pid) {
                panic!("Process `{pid}` may still be running. Use `exec --force`.");
            }
        }
        write_pid(&pid_file_path);

        // Execute the command
        let root_fs = root_fs_path(&self.container);
        execute_command(&self.command, &self.command_args, root_fs)
    }
}
