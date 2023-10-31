// https://distribution.github.io/distribution/spec/auth/token/

use crate::www_authenticate::WwwAuthenticate;

pub async fn pass_token_auth<F>(f: F) -> reqwest::Response
where
    F: Fn(&reqwest::Client) -> reqwest::RequestBuilder,
{
    let client = reqwest::Client::new();

    // Attempt to begin a push/pull operation with the registry.
    let resp = f(&client).send().await.unwrap();
    if resp.status().as_u16() != 401 {
        return resp;
    }

    // If the registry requires authorization it will return a `401 Unauthorized`` HTTP response with information on how to authenticate.
    let how = resp.headers().get("www-authenticate").unwrap();
    let how = how.to_str().unwrap();
    let how: WwwAuthenticate = how.parse().unwrap();
    assert_eq!(how.scheme(), "Bearer");

    // The registry client makes a request to the authorization service for a Bearer token.
    let mut query = String::new();
    let queries = how.pairs().iter().filter(|(k, _)| *k != "realm");
    for (i, (key, value)) in queries.enumerate() {
        if i != 0 {
            query.push('&');
        }
        query.push_str(key);
        query.push('=');
        query.push_str(value);
    }
    let token_url = format!("{}?{}", how.pairs().get("realm").unwrap(), query);

    // The authorization service returns an opaque Bearer token representing the client’s authorized access.
    let resp: models::TokenResponse = client
        .get(token_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // The client retries the original request with the Bearer token embedded in the request’s Authorization header.
    let authorization = format!("Bearer {}", resp.token());

    // The Registry authorizes the client by validating the Bearer token and the claim set embedded within it and begins the push/pull session as usual.
    f(&client)
        .header("Authorization", authorization)
        .send()
        .await
        .unwrap()
}

#[allow(dead_code)]
mod models {
    use getset::{CopyGetters, Getters};
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize, Getters, CopyGetters)]
    pub struct TokenResponse {
        #[getset(get = "pub")]
        token: String,
        #[getset(get = "pub")]
        access_token: String,
        #[getset(get_copy = "pub")]
        expires_in: usize,
        #[getset(get = "pub")]
        issued_at: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pass_token_auth() {
        let url = "https://registry.hub.docker.com/v2/";
        let resp = pass_token_auth(move |client| client.get(url)).await;
        dbg!(&resp);
        assert!(resp.status().is_success());
        dbg!(&resp.text().await.unwrap());
    }
}
