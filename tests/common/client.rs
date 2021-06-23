use reqwest::header::HeaderMap;

pub struct TestClient {
    pub base: String,
    pub client: reqwest::Client,
}

impl TestClient {
    pub fn clone_with_api_key(&self, api_key: String) -> TestClient {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", api_key).parse().unwrap(),
        );

        TestClient {
            base: self.base.clone(),
            client: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Building client"),
        }
    }

    pub fn get(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.get(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn post(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.post(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn put(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.put(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn delete(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client
            .delete(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn request(
        &self,
        method: reqwest::Method,
        url: impl AsRef<str>,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}/{}", self.base, url.as_ref()))
    }
}
