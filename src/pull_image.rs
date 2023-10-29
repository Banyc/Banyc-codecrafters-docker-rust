use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::token_auth::pass_token_auth;

const REGISTRY_BASE: &str = "https://registry.hub.docker.com/v2";
const ARCHITECTURE: &str = "amd64";
const MEDIA_TYPE_MANIFEST_LIST: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
const MEDIA_TYPE_DISTRIBUTION: &str = "application/vnd.docker.distribution.manifest.v2+json";
const MEDIA_TYPE_OCI: &str = "application/vnd.oci.image.manifest.v1+json";

pub async fn pull(image: &str) {
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
    dbg!(&resp);
    // dbg!(&resp.text().await.unwrap());
    let manifest_list: ImageManifestList = resp.json().await.unwrap();
    dbg!(&manifest_list);
    let manifest = &manifest_list
        .manifests
        .iter()
        .find(|manifest| manifest.platform.architecture == ARCHITECTURE)
        .unwrap();
    let (media_type, digest) = (&manifest.media_type, &manifest.digest);

    match media_type.as_str() {
        MEDIA_TYPE_DISTRIBUTION => {
            // https://distribution.github.io/distribution/spec/manifest-v2-2/#image-manifest
            handle_distribution(image_name, digest, MEDIA_TYPE_DISTRIBUTION).await
        }
        // https://github.com/opencontainers/image-spec/blob/main/manifest.md
        MEDIA_TYPE_OCI => handle_distribution(image_name, digest, MEDIA_TYPE_OCI).await,
        _ => panic!(),
    }
}

async fn handle_distribution(image_name: &str, digest: &str, accept: &str) {
    let url_manifest = format!("{REGISTRY_BASE}/library/{image_name}/manifests/{digest}");
    // let url_manifest = format!("{REGISTRY_BASE}/library/{image_name}/manifests/{image_version}");
    let resp = pass_token_auth(|client| client.get(&url_manifest).header("Accept", accept)).await;
    dbg!(&resp);
    let manifest: ImageManifest = resp.json().await.unwrap();
    dbg!(&manifest);
    // dbg!(&resp.text().await.unwrap());

    for layer in &manifest.layers {
        let digest = &layer.digest;

        pull_layer(image_name, digest).await;
    }
}

// https://distribution.github.io/distribution/spec/api/#pulling-a-layer
async fn pull_layer(image_name: &str, digest: &str) {
    let url_blob = format!("{REGISTRY_BASE}/library/{image_name}/blobs/{digest}");
    dbg!(&url_blob);
    let resp = pass_token_auth(|client| client.get(&url_blob)).await;
    dbg!(&resp);

    download(resp, digest).await;
}

async fn download(resp: reqwest::Response, digest: &str) {
    let bytes = resp.bytes().await.unwrap();
    let file_name = digest.split_once(':').unwrap().1;
    let tmp_dir = std::path::Path::new("/tmp/mydocker/layers");
    tokio::fs::create_dir_all(tmp_dir).await.unwrap();
    let file = tmp_dir.join(file_name);
    let mut file = tokio::fs::File::options()
        .create(true)
        .write(true)
        .open(file)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pull_distribution() {
        let image = "busybox:latest";
        pull(image).await;
    }

    #[tokio::test]
    async fn test_pull_oci() {
        let image = "ubuntu:latest";
        pull(image).await;
    }
}
