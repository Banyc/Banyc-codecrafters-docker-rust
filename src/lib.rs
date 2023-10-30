pub mod exec;
pub mod mounting;
pub mod pull_image;
pub mod run;
pub mod token_auth;
pub mod www_authenticate;

const CONTAINERS: &str = "/tmp/mydocker/containers";

fn container_dir(name: &str) -> std::path::PathBuf {
    std::path::Path::new(CONTAINERS).join(name)
}

fn pid_file_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(CONTAINERS).join(name).join("pid")
}

fn root_fs_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(CONTAINERS).join(name).join("rootfs")
}

fn read_pid(pid_file_path: impl AsRef<std::path::Path>) -> Option<usize> {
    if !pid_file_path.as_ref().exists() {
        return None;
    }

    use std::io::Read;

    let file = std::fs::File::options()
        .read(true)
        .open(&pid_file_path)
        .unwrap();
    let mut file = std::io::BufReader::new(file);
    let mut pid = String::new();
    file.read_to_string(&mut pid).unwrap();
    let pid: usize = pid.parse().unwrap();
    Some(pid)
}

fn write_pid(pid_file_path: impl AsRef<std::path::Path>) {
    use std::io::Write;

    let pid = std::process::id();
    let mut file = std::fs::File::options()
        .create(true)
        .write(true)
        .open(&pid_file_path)
        .unwrap();
    file.write_all(format!("{pid}").as_bytes()).unwrap();
}

fn execute_command(
    command: impl AsRef<std::path::Path> + std::fmt::Debug,
    command_args: &[String],
    root: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Chroot the root directory
    std::os::unix::fs::chroot(root).unwrap();
    std::env::set_current_dir("/").unwrap();

    // The calling process is not moved into the new namespace.
    // The first child created by the calling process will have the process ID 1 and will assume the role of init(1) in the new namespace.
    #[cfg(target_os = "linux")]
    {
        let res = unsafe { libc::unshare(libc::CLONE_NEWPID) };
        if res != 0 {
            std::process::exit(res);
        }
    }

    // Execute the command
    let mut child = std::process::Command::new(command.as_ref())
        .args(command_args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!(
                "Tried to run '{:?}' with arguments {:?}",
                command, command_args
            )
        })?;

    // Wait for the child to exit
    let exit_status = child.wait().unwrap();

    // Return exit code
    if let Some(code) = exit_status.code() {
        std::process::exit(code);
    }
    Ok(())
}
