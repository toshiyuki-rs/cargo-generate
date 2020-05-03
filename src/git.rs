use crate::cargo::core::GitReference;
use crate::cargo::sources::git::GitRemote;
use crate::cargo::util::config::Config;
use failure;
use git2::{Repository as GitRepository, RepositoryInitOptions};
use quicli::prelude::*;
use remove_dir_all::remove_dir_all;
use std::env::current_dir;
use std::path::Path;
use std::path::PathBuf;
use tempfile::Builder;
use url::{ParseError, Url};

pub struct GitConfig {
    remote: Url,
    branch: GitReference,
}

impl GitConfig {
    pub fn new(git: String, branch: String) -> Result<Self, failure::Error> {
        let remote = match Url::parse(&git) {
            Ok(u) => u,
            Err(ParseError::RelativeUrlWithoutBase) => {
                let given_path = Path::new(&git);
                let mut git_path = PathBuf::new();
                if given_path.is_relative() {
                    git_path.push(current_dir()?);
                    git_path.push(given_path);
                } else {
                    git_path.push(&git)
                }
                let rel = "file://".to_string() + &git_path.to_str().unwrap_or("").to_string();
                Url::parse(&rel)?
            }
            Err(_) => return Err(format_err!("Failed parsing git remote: {}", &git)),
        };

        Ok(GitConfig {
            remote,
            branch: GitReference::Branch(branch),
        })
    }
}

pub fn create(project_dir: &PathBuf, args: GitConfig) -> Result<(), failure::Error> {
    let temp = Builder::new()
        .prefix(project_dir.to_str().unwrap_or("cargo-generate"))
        .tempdir()?;

    let config = Config::default();
    let config = match config {
        Ok(conf) => conf,
        Err(err) => match err.downcast::<failure::Error>() {
            Ok(e) => return Err(e),
            Err(e) => panic!("unexpected error")
        }
    };
    let remote = GitRemote::new(&args.remote);
    let rem_res = remote.checkout(&temp.path(), &args.branch, &config);

    let (db, rev) = match rem_res {
        Ok((db, rev)) => (db, rev),
        Err(err) => match err.downcast::<failure::Error>() {
            Ok(e) => return Err(e),
            Err(e) => panic!("unexpected error")
        }
    };

    // This clones the remote and handles all the submodules
    let db_res = db.copy_to(rev, project_dir.as_path(), &config);

    match db_res {
        Ok(co) => (),
        Err(err) => match err.downcast::<failure::Error>() {
            Ok(e) => return Err(e),
            Err(e) => panic!("unexpected error")
        }
    };

    Ok(())
}

pub fn remove_history(project_dir: &PathBuf) -> Result<(), failure::Error> {
    remove_dir_all(project_dir.join(".git")).context("Error cleaning up cloned template")?;
    Ok(())
}

pub fn init(project_dir: &PathBuf, branch: &str) -> Result<GitRepository, failure::Error> {
    Ok(GitRepository::init_opts(
        project_dir,
        RepositoryInitOptions::new()
            .bare(false)
            .initial_head(branch),
    )
    .context("Couldn't init new repository")?)
}
