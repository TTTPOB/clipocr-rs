use async_trait::async_trait;
use log::{debug, error, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct OcrConfig {
    app_id: String,
    api_key: String,
    sec_key: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OcrState {
    access_token: String,
    expire_time: u64,
}

#[derive(Serialize, Deserialize)]
struct AccessTokenResponse {
    refresh_token: String,
    expires_in: i64,
    scope: String,
    session_key: String,
    access_token: String,
    session_secret: String,
}
impl From<AccessTokenResponse> for OcrState {
    fn from(resp: AccessTokenResponse) -> Self {
        let expire_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + resp.expires_in as u64;
        Self {
            access_token: resp.access_token,
            expire_time,
        }
    }
}

impl OcrConfig {
    pub fn new(app_id: String, api_key: String, sec_key: String) -> Self {
        Self {
            app_id,
            api_key,
            sec_key,
        }
    }
    pub fn from_yaml(path: &str) -> Self {
        let config = std::fs::read_to_string(path).unwrap();
        let config: OcrConfig = serde_yaml::from_str(&config).unwrap();
        config
    }
    pub fn to_yaml(&self, path: &str) {
        let config = serde_yaml::to_string(self).unwrap();
        std::fs::write(path, config).unwrap();
    }
    async fn refresh_state(&self) -> OcrState {
        let url = format!("https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={}&client_secret={}", self.api_key, self.sec_key);
        let resp = reqwest::get(&url).await.unwrap();
        let resp: AccessTokenResponse = resp.json().await.unwrap();
        resp.into()
    }
    async fn refresh_state_if_expired(&self, state: &OcrState) -> OcrState {
        // get current unix timestamp
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if state.expire_time < now {
            self.refresh_state().await
        } else {
            state.clone()
        }
    }
    pub async fn get_valid_state(&self, path: &str) -> OcrState {
        // determine if path exists
        if !std::path::Path::new(path).exists() {
            let new_state = self.refresh_state().await;
            new_state.to_yaml(path);
            return new_state;
        }
        let state = OcrState::from_yaml(path);
        debug!("expire time in state file: {}", state.expire_time);
        let new_state = self.refresh_state_if_expired(&state).await;
        if state != new_state {
            new_state.to_yaml(path);
        }
        state
    }
}

impl OcrState {
    pub fn new(access_token: String, expire_time: u64) -> Self {
        Self {
            access_token,
            expire_time,
        }
    }
    fn from_yaml(path: &str) -> Self {
        let state = std::fs::read_to_string(path).unwrap();
        let state: OcrState = serde_yaml::from_str(&state).unwrap();
        state
    }
    fn to_yaml(&self, path: &str) {
        let state = serde_yaml::to_string(self).unwrap();
        std::fs::write(path, state).unwrap();
    }
}

pub enum BaiduOcrApis {
    GeneralBasic(BaiduGeneralBasic),
    AccurateBasic(BaiduAccurateBasic),
}
pub struct BaiduGeneralBasic {
    access_token: String,
}
impl BaiduGeneralBasic {
    const BASEURL: &'static str = "https://aip.baidubce.com/rest/2.0/ocr/v1/general_basic";
    pub fn from_state(state: &OcrState) -> Self {
        Self {
            access_token: state.access_token.clone(),
        }
    }
}
pub struct BaiduAccurateBasic {
    access_token: String,
}
impl BaiduAccurateBasic {
    const BASEURL: &'static str = "https://aip.baidubce.com/rest/2.0/ocr/v1/accurate_basic";
    pub fn from_state(state: &OcrState) -> Self {
        Self {
            access_token: state.access_token.clone(),
        }
    }
}

#[async_trait]
pub trait OcrApi {
    type OcrResult: OcrResult;
    fn url(&self) -> String;
    async fn get_result(&self, image_base64: &str) -> Self::OcrResult {
        let url = self.url();
        let resp = reqwest::Client::new()
            .post(&url)
            .form(&[("image", image_base64)])
            .send()
            .await
            .unwrap();
        let resp_text = resp.text().await.unwrap();
        let result: Self::OcrResult = serde_json::from_str(&resp_text).unwrap_or_else(|_| {
            error!("failed to parse response: {}", resp_text);
            panic!();
        });
        result
    }
    async fn get_text_result(&self, image_base64: &str) -> Vec<String> {
        let result = self.get_result(image_base64).await;
        result.extract_text()
    }
}

impl OcrApi for BaiduOcrApis {
    type OcrResult = BaiduOcrResult;
    fn url(&self) -> String {
        match self {
            BaiduOcrApis::GeneralBasic(api) => format!(
                "{}?access_token={}",
                BaiduGeneralBasic::BASEURL,
                api.access_token
            ),
            BaiduOcrApis::AccurateBasic(api) => format!(
                "{}?access_token={}",
                BaiduAccurateBasic::BASEURL,
                api.access_token
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BaiduOcrResult {
    log_id: u64,
    words_result_num: u32,
    words_result: Vec<WordResult>,
}
#[derive(Serialize, Deserialize, Debug)]
struct WordResult {
    words: String,
}
// deserialize
pub trait OcrResult: DeserializeOwned {
    fn extract_text(&self) -> Vec<String>;
}
impl OcrResult for BaiduOcrResult {
    fn extract_text(&self) -> Vec<String> {
        self.words_result.iter().map(|x| x.words.clone()).collect()
    }
}
