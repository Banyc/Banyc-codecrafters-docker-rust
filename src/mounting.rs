use crate::overlay_fs_lower_dir;

pub fn mount(container_name: &str) {
    use crate::root_fs_path;

    // Mount root_fs
    {
        if mount_layers(container_name).is_err() {
            // We have to mount tmpfs inside a container
            // But the writable layers will not survive reboots
            mount_writable_tmp_fs(container_name);
            mount_layers(container_name).unwrap();
        }
    }

    // Mount `/proc`
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

fn mount_writable_tmp_fs(container_name: &str) {
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

fn mount_layers(container_name: &str) -> nix::Result<()> {
    use crate::{overlay_fs_upper_dir, overlay_fs_work_dir, root_fs_path};

    let mut lower_dir_string = String::new();
    for (i, layer) in overlay_fs_lower_dir(container_name)
        .read_dir()
        .unwrap()
        .enumerate()
    {
        let layer = layer.unwrap();
        if i != 0 {
            lower_dir_string.push(':');
        }
        lower_dir_string.push_str(layer.path().to_str().unwrap());
    }

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
