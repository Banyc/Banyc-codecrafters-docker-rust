use async_compression::tokio::bufread::GzipDecoder;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::token_auth::pass_token_auth;

const REGISTRY_BASE: &str = "https://registry.hub.docker.com/v2";
const ARCHITECTURE: &str = "amd64";
const MEDIA_TYPE_MANIFEST_LIST: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
const MEDIA_TYPE_DISTRIBUTION: &str = "application/vnd.docker.distribution.manifest.v2+json";
const MEDIA_TYPE_OCI: &str = "application/vnd.oci.image.manifest.v1+json";
const LAYER_DIR: &str = "/tmp/mydocker/layers";

pub async fn pull(image: &str, root: impl AsRef<std::path::Path>) {
    let (image_name, image_version) = image.split_once(':').unwrap();
    // https://distribution.github.io/distribution/spec/api/#pulling-an-image-manifest
    let url_manifests = format!("{REGISTRY_BASE}/library/{image_name}/manifests/{image_version}");

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
        .find(|manifest| manifest.platform.architecture == ARCHITECTURE)
        .unwrap();
    let (media_type, digest) = (&manifest.media_type, &manifest.digest);

    match media_type.as_str() {
        MEDIA_TYPE_DISTRIBUTION => {
            // https://distribution.github.io/distribution/spec/manifest-v2-2/#image-manifest
            handle_manifest(image_name, digest, MEDIA_TYPE_DISTRIBUTION, root).await
        }
        // https://github.com/opencontainers/image-spec/blob/main/manifest.md
        MEDIA_TYPE_OCI => handle_manifest(image_name, digest, MEDIA_TYPE_OCI, root).await,
        _ => panic!("{media_type}"),
    }
}

async fn handle_manifest(
    image_name: &str,
    digest: &str,
    accept: &str,
    root: impl AsRef<std::path::Path>,
) {
    let url_manifest = format!("{REGISTRY_BASE}/library/{image_name}/manifests/{digest}");
    // let url_manifest = format!("{REGISTRY_BASE}/library/{image_name}/manifests/{image_version}");
    let resp = pass_token_auth(|client| client.get(&url_manifest).header("Accept", accept)).await;
    // dbg!(&resp);
    let manifest: ImageManifest = resp.json().await.unwrap();
    // dbg!(&manifest);
    // dbg!(&resp.text().await.unwrap());

    for (i, layer) in manifest.layers.iter().enumerate() {
        let digest = &layer.digest;

        let file_path = pull_layer(image_name, i, digest).await;
        let tar_gz = tokio::fs::File::options()
            .read(true)
            .open(file_path)
            .await
            .unwrap();
        let tar_gz = tokio::io::BufReader::new(tar_gz);
        let tar = GzipDecoder::new(tar_gz);
        let mut archive = tokio_tar::Archive::new(tar);
        archive.unpack(&root).await.unwrap();
    }
}

// https://distribution.github.io/distribution/spec/api/#pulling-a-layer
async fn pull_layer(image_name: &str, layer_index: usize, digest: &str) -> std::path::PathBuf {
    let url_blob = format!("{REGISTRY_BASE}/library/{image_name}/blobs/{digest}");
    // dbg!(&url_blob);
    let resp = pass_token_auth(|client| client.get(&url_blob)).await;
    // dbg!(&resp);

    download(resp, image_name, layer_index, digest).await
}

async fn download(
    resp: reqwest::Response,
    image_name: &str,
    layer_index: usize,
    digest: &str,
) -> std::path::PathBuf {
    let bytes = resp.bytes().await.unwrap();
    let layer_dir = std::path::Path::new(LAYER_DIR);
    tokio::fs::create_dir_all(layer_dir).await.unwrap();
    let file_path = layer_dir.join(format!("{image_name}.{layer_index}.{digest}"));
    let mut file = tokio::fs::File::options()
        .create(true)
        .write(true)
        .open(&file_path)
        .await
        .unwrap();
    file.write_all(&bytes).await.unwrap();
    file_path
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

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    const ROOT: &str = "/tmp/mydocker/root";

    #[tokio::test]
    #[serial]
    async fn test_pull_distribution() {
        let image = "busybox:latest";
        let _ = tokio::fs::remove_dir_all(ROOT).await;
        pull(image, ROOT).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_pull_oci() {
        let image = "ubuntu:latest";
        let _ = tokio::fs::remove_dir_all(ROOT).await;
        pull(image, ROOT).await;
    }
}
