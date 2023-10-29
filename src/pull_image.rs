use std::borrow::Cow;

use async_compression::tokio::bufread::GzipDecoder;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::token_auth::pass_token_auth;

const REGISTRY_BASE: &str = "https://registry.hub.docker.com/v2";
const MEDIA_TYPE_MANIFEST_LIST: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
const MEDIA_TYPE_DISTRIBUTION: &str = "application/vnd.docker.distribution.manifest.v2+json";
const MEDIA_TYPE_OCI: &str = "application/vnd.oci.image.manifest.v1+json";
const LAYER_DIR: &str = "/tmp/mydocker/layers";
const UNPACK_TEMP_DIR: &str = "/tmp/mydocker/unpack/";

pub async fn pull(image: &str, root: std::path::PathBuf) {
    let (image_name, image_version) = image.split_once(':').unwrap();
    let image_name: Cow<'_, str> = match image_name.contains('/') {
        true => image_name.into(),
        false => format!("library/{}", image_name).into(),
    };
    // https://distribution.github.io/distribution/spec/api/#pulling-an-image-manifest
    let url_manifests = format!("{REGISTRY_BASE}/{image_name}/manifests/{image_version}");

    // // https://distribution.github.io/distribution/spec/manifest-v2-2/#manifest-list
    let resp = pass_token_auth(|client| {
        client
            .get(&url_manifests)
            .header("Accept", MEDIA_TYPE_MANIFEST_LIST)
    })
    .await;
    // dbg!(&resp);
    // dbg!(&resp.text().await.unwrap());
    let manifest_list: ImageManifestList = resp.json().await.unwrap();
    // dbg!(&manifest_list);
    let manifest = &manifest_list
        .manifests
        .iter()
        .find(|manifest| manifest.platform.architecture == docker_arch())
        .unwrap();
    let (media_type, digest) = (&manifest.media_type, &manifest.digest);

    match media_type.as_str() {
        MEDIA_TYPE_DISTRIBUTION => {
            // https://distribution.github.io/distribution/spec/manifest-v2-2/#image-manifest
            handle_manifest(&image_name, digest, MEDIA_TYPE_DISTRIBUTION, root).await
        }
        // https://github.com/opencontainers/image-spec/blob/main/manifest.md
        MEDIA_TYPE_OCI => handle_manifest(&image_name, digest, MEDIA_TYPE_OCI, root).await,
        _ => panic!("{media_type}"),
    }
}

async fn handle_manifest(image_name: &str, digest: &str, accept: &str, root: std::path::PathBuf) {
    let url_manifest = format!("{REGISTRY_BASE}/{image_name}/manifests/{digest}");
    // let url_manifest = format!("{REGISTRY_BASE}/library/{image_name}/manifests/{image_version}");
    let resp = pass_token_auth(|client| client.get(&url_manifest).header("Accept", accept)).await;
    // dbg!(&resp);
    let manifest: ImageManifest = resp.json().await.unwrap();
    // dbg!(&manifest);
    // dbg!(&resp.text().await.unwrap());

    let unpack_dir = std::path::PathBuf::from(UNPACK_TEMP_DIR).join(root.file_name().unwrap());

    for (i, layer) in manifest.layers.iter().enumerate() {
        let digest = &layer.digest;

        let _ = tokio::fs::remove_dir_all(&unpack_dir).await;
        tokio::fs::create_dir_all(&unpack_dir).await.unwrap();

        let file_path = pull_layer(image_name, i, digest).await;
        let tar_gz = tokio::fs::File::options()
            .read(true)
            .open(file_path)
            .await
            .unwrap();
        let tar_gz = tokio::io::BufReader::new(tar_gz);
        let tar = GzipDecoder::new(tar_gz);
        let mut archive = tokio_tar::Archive::new(tar);
        archive.unpack(&unpack_dir).await.unwrap();
        let root = root.clone();
        let unpack_dir = unpack_dir.clone();
        tokio::task::spawn_blocking(move || {
            move_dir_recursive(&unpack_dir, &root).unwrap();
        })
        .await
        .unwrap();
    }

    let _ = tokio::fs::remove_dir_all(UNPACK_TEMP_DIR).await;
}

// https://distribution.github.io/distribution/spec/api/#pulling-a-layer
async fn pull_layer(image_name: &str, layer_index: usize, digest: &str) -> std::path::PathBuf {
    let layer_dir = std::path::Path::new(LAYER_DIR);
    tokio::fs::create_dir_all(layer_dir).await.unwrap();
    let (image_name_left, image_name_right) = image_name.split_once('/').unwrap();
    let file_path = layer_dir.join(format!(
        "{image_name_left}.{image_name_right}.{layer_index}.{digest}.tar.gz"
    ));
    if file_path.exists() {
        // Use cached layer
        return file_path;
    }

    let url_blob = format!("{REGISTRY_BASE}/{image_name}/blobs/{digest}");
    // dbg!(&url_blob);
    let resp = pass_token_auth(|client| client.get(&url_blob)).await;
    // dbg!(&resp);

    download(resp, &file_path).await;
    file_path
}

async fn download(resp: reqwest::Response, file_path: impl AsRef<std::path::Path>) {
    let bytes = resp.bytes().await.unwrap();

    let mut file = tokio::fs::File::options()
        .create(true)
        .write(true)
        .open(&file_path)
        .await
        .unwrap();
    file.write_all(&bytes).await.unwrap();
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageManifestList {
    // schema_version: usize,
    // media_type: String,
    manifests: Vec<ImagePlatformManifest>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImagePlatformManifest {
    media_type: String,
    // size: usize,
    digest: String,
    platform: ImagePlatform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImagePlatform {
    architecture: String,
    // os: String,
    // #[serde(rename = "os.version")]
    // os_version: Option<String>,
    // #[serde(rename = "os.features")]
    // os_features: Option<Vec<String>>,
    // variant: Option<String>,
    // features: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageManifest {
    // schema_version: usize,
    // media_type: String,
    // config: ImageConfig,
    layers: Vec<ImageLayer>,
}

// #[derive(Debug, Clone, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct ImageConfig {
//     media_type: String,
//     size: usize,
//     digest: String,
// }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageLayer {
    // media_type: String,
    // size: usize,
    digest: String,
    // urls: Option<Vec<String>>,
}

fn docker_arch() -> &'static str {
    let arch = std::env::consts::ARCH;
    match arch {
        "x86" => "i386",
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => arch,
    }
}

fn move_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    use std::fs;

    if src.is_dir() {
        fs::create_dir_all(dst)?;

        for entry in src.read_dir()? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                move_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::rename(&src_path, &dst_path)?;
            }
        }
    } else {
        fs::rename(src, dst)?;
    }

    fs::remove_dir_all(src)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    const ROOT: &str = "/tmp/mydocker/test/rootfs";

    #[tokio::test]
    #[serial]
    async fn test_pull_distribution() {
        let image = "busybox:latest";
        let _ = tokio::fs::remove_dir_all(ROOT).await;
        pull(image, ROOT.into()).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_pull_oci() {
        let image = "ubuntu:latest";
        let _ = tokio::fs::remove_dir_all(ROOT).await;
        pull(image, ROOT.into()).await;
    }
}
