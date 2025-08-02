use anyhow::{Context, Result};
use git2::{Repository };

pub struct RepoExt(pub Repository);

impl RepoExt {
    pub fn open(path: &str) -> Result<Self> {
        Repository::discover(path)
            .with_context(|| format!("cannot open repo at {path}"))
            .map(Self)
    }
    pub fn repo(&self) -> &Repository { &self.0 }

}

