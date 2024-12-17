use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Replace {
    Text(String),
    Line(String),
    Regex(String),
    LinesBetween(String, String),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum With {
    File {
        file: std::path::PathBuf,
    },
    Jinja {
        template: std::path::PathBuf,
        values: std::path::PathBuf,
    },
    String {
        string: String,
    },
    Pattern {
        pattern: String,
    },
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Which {
    First,
    #[serde(rename = "first in line")]
    FirstInLine,
    #[default]
    Every,
}

#[derive(Deserialize)]
pub struct Rule {
    pub replace: Replace,
    pub with: With,

    #[serde(default)]
    pub keep_spaces: bool,

    #[serde(default)]
    pub which: Which,
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(rename = "rule")]
    pub rules: Vec<Rule>,
}
