#[cfg(target_os = "linux")]
pub fn mount(container_name: &str) {
    // Mount `/proc`

    use crate::root_fs_path;
    {
        let root_fs = root_fs_path(container_name);
        let proc_dir = root_fs.join("proc");
        std::fs::create_dir_all(&proc_dir).unwrap();

        nix::mount::mount(
            Some("/proc"),
            &proc_dir,
            Some("proc"),
            nix::mount::MsFlags::empty(),
            None::<&std::path::Path>,
        )
        .unwrap();
    }
}

#[cfg(not(target_os = "linux"))]
pub fn mount(_root_fs: impl AsRef<std::path::Path>) {}

#[cfg(target_os = "linux")]
pub fn unmount(container_name: &str) {
    use crate::{overlay_fs_writable_layers_dir, root_fs_path};

    // Unmount `/proc` in `root_fs`
    {
        let root_fs = root_fs_path(container_name);
        let proc_dir = root_fs.join("proc");
        let _ = nix::mount::umount2(&proc_dir, nix::mount::MntFlags::MNT_FORCE);
    }

    // Unmount writable dir of overlay fs
    {
        let writable = overlay_fs_writable_layers_dir(container_name);
        let _ = nix::mount::umount2(&writable, nix::mount::MntFlags::MNT_FORCE);
    }

    // Unmount `root_fs`
    {
        let root_fs = root_fs_path(container_name);
        let _ = nix::mount::umount2(&root_fs, nix::mount::MntFlags::MNT_FORCE);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn unmount(_container_name: &str) {}

#[cfg(target_os = "linux")]
pub fn mount_writable_tmp_fs(container_name: &str) {
    use crate::overlay_fs_writable_layers_dir;

    // https://stackoverflow.com/a/67208735/9920172
    let writable = overlay_fs_writable_layers_dir(container_name);
    std::fs::create_dir_all(&writable).unwrap();
    nix::mount::mount(
        Some("tmpfs"),
        &writable,
        Some("tmpfs"),
        nix::mount::MsFlags::empty(),
        None::<&std::path::Path>,
    )
    .unwrap();
}

#[cfg(not(target_os = "linux"))]
pub fn mount_writable_tmp_fs(_container_name: &str) {}

#[cfg(target_os = "linux")]
pub fn mount_layers(container_name: &str, lower_dir_string: &str) -> nix::Result<()> {
    use crate::{overlay_fs_upper_dir, overlay_fs_work_dir, root_fs_path};

    let upper_dir = overlay_fs_upper_dir(container_name);
    std::fs::create_dir_all(&upper_dir).unwrap();
    let upper_dir = upper_dir.to_str().unwrap();
    // https://unix.stackexchange.com/a/330166
    let work_dir = overlay_fs_work_dir(container_name);
    std::fs::create_dir_all(&work_dir).unwrap();
    let work_dir = work_dir.to_str().unwrap();
    let root_fs = root_fs_path(container_name);
    std::fs::create_dir_all(&root_fs).unwrap();

    let overlay_o = format!("lowerdir={lower_dir_string},upperdir={upper_dir},workdir={work_dir}",);
    // dbg!(&overlay_o);
    // dbg!(&root_fs);
    nix::mount::mount(
        Some("overlay"),
        &root_fs,
        Some("overlay"),
        nix::mount::MsFlags::empty(),
        Some(overlay_o.as_str()),
    )
}

#[cfg(not(target_os = "linux"))]
pub fn mount_layers(_container_name: &str, _lower_dir_string: &str) -> nix::Result<()> {
    Ok(())
}
