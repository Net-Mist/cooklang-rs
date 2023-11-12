use std::{
    fs::{create_dir_all, read_to_string, write},
    path::PathBuf,
};

use anyhow::{bail, Context, Result};
use dashmap::{DashMap, DashSet};
use serde::Deserialize;
use serde_yaml::to_string;
use thiserror::Error;
use tower_lsp::{
    lsp_types::{CompletionItem, CompletionItemKind, WorkspaceFolder},
    Client,
};
use tracing::{info, warn};

use crate::parser::CooklangParser;

#[derive(Deserialize, Default)]
struct Config {
    cookwares: Option<Vec<String>>,
    ingredients: Option<Vec<String>>,
}

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Can't find config dir")]
    NoConfigDir,
}

#[derive(Debug)]
pub(crate) struct Backend {
    pub(crate) client: Client,

    /// set of ingredients loaded from conf
    pub(crate) ingredients_user: DashSet<String>,

    /// set of cookwares loaded from conf
    pub(crate) cookwares_user: DashSet<String>,

    /// map of set of ingredient for each files in the workspace
    pub(crate) ingredients_files: DashMap<PathBuf, DashSet<String>>,

    /// map of set of cookwares for each files in the workspace
    pub(crate) cookwares_files: DashMap<PathBuf, DashSet<String>>,

    /// map of vector of step lines for each files in the workspace
    pub(crate) step_lines: DashMap<PathBuf, Vec<usize>>,

    /// Content of opened_files,
    pub(crate) file_content: DashMap<PathBuf, String>,
}

impl Backend {
    /// Creates a new [`Backend`].
    pub(crate) fn new(client: Client) -> Backend {
        let config = load_conf().unwrap_or_else(|e| {
            warn!("{e:#?}");
            Config::default()
        });

        Backend {
            client,
            ingredients_user: config.ingredients.unwrap_or_default().into_iter().collect(),
            cookwares_user: config.cookwares.unwrap_or_default().into_iter().collect(),
            ingredients_files: DashMap::new(),
            cookwares_files: DashMap::new(),
            file_content: DashMap::new(),
            step_lines: DashMap::new(),
        }
    }

    pub(crate) fn index_file(&self, path: PathBuf, content: String) {
        self.file_content.insert(path, content);
    }

    pub(crate) fn process_str(&self, text: &str, file: &PathBuf) -> Result<()> {
        let (ingredients, cookwares, step_lines) =
            process_str(text).context(format!("can't parse {file:?}"))?;
        self.ingredients_files.insert(file.to_owned(), ingredients);
        self.cookwares_files.insert(file.to_owned(), cookwares);
        self.step_lines.insert(file.to_owned(), step_lines);
        Ok(())
    }

    /// Check if the file is a valid cooklang, then load it and add it
    /// ingredients and wookwares to the backend.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file is invalid or can't be parsed.
    pub(crate) fn process_file(&self, file: &PathBuf) -> Result<()> {
        let file_extension = file
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        if file_extension != "cook" {
            bail!("can't process {file:?}, not a cooklang file")
        }

        let content = read_to_string(file).context(format!("Can't read {:?}", file.to_str()))?;

        self.process_str(&content, file)
    }

    pub(crate) fn process_workspace(&self, workspace: &WorkspaceFolder) -> Result<()> {
        info!("parse workspace {}", to_string(&workspace)?);
        let path = uri_to_pathbuf(&workspace.uri)?;

        let mut dir_to_explore = vec![path];
        while let Some(dir) = dir_to_explore.pop() {
            for p in dir.read_dir().unwrap() {
                let p = p?.path();
                if p.is_dir() {
                    dir_to_explore.push(p);
                    continue;
                }
                info!("parse {}", p.as_os_str().to_str().unwrap_or_default());
                self.process_file(&p).unwrap_or_else(|e| {
                    warn!("{e:#?}");
                });
            }
        }
        Ok(())
    }

    pub(crate) fn completions(&self) -> Vec<CompletionItem> {
        // TODO for now we can return all ingredient and cookware and let the front sort them.
        // this strategy is valid because of the unique "#" and "@" in their names

        let ingredients_files = self
            .ingredients_files
            .iter()
            .flat_map(|k_v| k_v.value().to_owned());

        let cookwares_files = self
            .cookwares_files
            .iter()
            .flat_map(|k_v| k_v.value().to_owned());

        #[allow(clippy::unnecessary_to_owned)]
        self.ingredients_user
            .to_owned()
            .into_iter()
            .chain(self.cookwares_user.to_owned())
            .chain(cookwares_files)
            .chain(ingredients_files)
            .map(|v| CompletionItem {
                label: v.to_string(),
                detail: Some(v.to_string()),
                sort_text: Some(v.to_string()),
                kind: Some(CompletionItemKind::VARIABLE),
                ..CompletionItem::default()
            })
            .collect()
    }
}

pub(crate) fn uri_to_pathbuf(uri: &tower_lsp::lsp_types::Url) -> Result<PathBuf> {
    let scheme = uri.scheme();
    let path = uri.path();
    if scheme != "file" {
        bail!("uri is not a file")
    }
    Ok(PathBuf::from(path))
}

fn load_conf() -> Result<Config> {
    let mut config_dir = dirs::config_dir().ok_or(BackendError::NoConfigDir)?;
    config_dir.push("cooklang");
    config_dir.push("lsp.conf");

    if !config_dir.exists() {
        // create parents
        let parents = config_dir.parent().unwrap();
        create_dir_all(parents)?;
        write(&config_dir, "")?
    }

    let config = read_to_string(config_dir).context("can't read conf")?;
    let config: Config = toml::from_str(&config).context("can't parse config")?;

    Ok(config)
}

/// Parse the input text and extract the ingredients, the cookwares and the step lines.
///
/// # Errors
///
/// This function will return an error if the text is not valid cooklang.
fn process_str(text: &str) -> Result<(DashSet<String>, DashSet<String>, Vec<usize>)> {
    let mut parser = CooklangParser::new();
    let parsed = parser.parse(text);

    Ok((
        parsed
            .ingredients
            .iter()
            .map(|part| "@".to_string() + &part)
            .collect(),
        parsed
            .cookwares
            .iter()
            .map(|part| "#".to_string() + &part)
            .collect(),
        parsed.step_lines,
    ))
}
