#[cfg(target_os = "linux")]
pub fn mount(root_fs: impl AsRef<std::path::Path>) {
    // Mount `/proc`
    {
        let proc_dir = root_fs.as_ref().join("proc");
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
pub fn unmount(root_fs: impl AsRef<std::path::Path>) {
    // Unmount `/proc` in `root_fs`
    {
        let proc_dir = root_fs.as_ref().join("proc");
        let _ = nix::mount::umount2(&proc_dir, nix::mount::MntFlags::MNT_FORCE);
    }

    // Unmount `root_fs`
    {
        let _ = nix::mount::umount2(root_fs.as_ref(), nix::mount::MntFlags::MNT_FORCE);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn unmount(_root_fs: impl AsRef<std::path::Path>) {}

#[cfg(target_os = "linux")]
pub fn mount_layers(
    lower_dir_string: &str,
    upper_dir: &str,
    work_dir: &str,
    root_fs: impl AsRef<std::path::Path>,
) {
    let overlay_o = format!("lowerdir={lower_dir_string},upperdir={upper_dir},workdir={work_dir}",);
    // dbg!(&overlay_o);
    // dbg!(&root_fs);
    nix::mount::mount(
        Some("overlay"),
        root_fs.as_ref(),
        Some("overlay"),
        nix::mount::MsFlags::empty(),
        Some(overlay_o.as_str()),
    )
    .unwrap();
}

#[cfg(not(target_os = "linux"))]
pub fn mount_layers(
    _lower_dir_string: &str,
    _upper_dir: &str,
    _work_dir: &str,
    _root_fs: impl AsRef<std::path::Path>,
) {
}
