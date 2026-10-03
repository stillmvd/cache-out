use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Chromium,
    Firefox,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Browser {
    pub id: String,
    pub name: String,
    pub family: Family,
    pub process: String,
    pub profiles: Vec<Profile>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub domain: String,
    pub cookies: u32,
    pub history_urls: u32,
    pub visits: u32,
    pub downloads: u32,
    pub storage_bytes: u64,
    pub site_cache_bytes: u64,
    pub last_visit: Option<i64>,
    pub last_cookie_access: Option<i64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FormField {
    pub name: String,
    pub entries: u32,
    pub last_used: Option<i64>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileScan {
    pub sites: Vec<Site>,
    pub cache_bytes: u64,
    pub forms: Vec<FormField>,
    pub addresses: u32,
    pub locked: Vec<String>,
    pub from_shadow: bool,
}
