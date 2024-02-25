use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Config {
    pub site_url: String,
    pub rss: Option<RSSConfig>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct RSSConfig {
    pub title: String,
    pub link: String,
    pub description: String,
    pub language: Option<String>,
    pub copyright: Option<String>,
    pub managing_editor: Option<String>,
    pub webmaster: Option<String>,
    pub categories: Option<Vec<Category>>,
    pub ttl: Option<u32>,
    pub image: Option<Image>,
    pub rating: Option<String>,
    pub text_input: Option<TextInput>,
    pub skip_hours: Option<Vec<String>>,
    pub skip_days: Option<Vec<String>>
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Category {
    pub name: String,
    pub domain: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Image {
    pub url: String,
    pub title: String,
    pub link: String,
    pub width: Option<String>,
    pub height: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TextInput {
    pub title: String,
    pub description: String,
    pub name: String,
    pub link: String,
}
