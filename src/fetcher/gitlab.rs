use std::{cell::OnceCell, fmt::Write};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
    impl_fetcher,
    simple::{SimpleFetcher, SimpleGitFetcher},
    Url,
};

pub struct FetchFromGitLab<'a> {
    pub host: Option<&'a str>,
    pub group: OnceCell<&'a str>,
}
impl_fetcher!(FetchFromGitLab<'a>);

impl<'a> FetchFromGitLab<'a> {
    pub fn new(host: Option<&'a str>) -> Self {
        Self {
            host,
            group: OnceCell::new(),
        }
    }
}

#[derive(Deserialize)]
struct Commit {
    id: String,
}

impl<'a> SimpleFetcher<'a, 2> for FetchFromGitLab<'a> {
    const KEYS: [&'static str; 2] = ["owner", "repo"];
    const NAME: &'static str = "fetchFromGitLab";
    const SUBMODULES_KEY: Option<&'static str> = Some("fetchSubmodules");

    fn host(&self) -> Option<&str> {
        self.host
    }

    fn group(&self) -> Option<&str> {
        self.group.get().copied()
    }

    fn get_values(&self, url: &'a Url) -> Option<[&'a str; 2]> {
        let mut xs = url.path_segments();
        let x = xs.next()?;
        let y = xs.next()?;
        Some(match xs.next() {
            None | Some("" | "-") => [x, y.strip_suffix(".git").unwrap_or(y)],
            Some(z) => {
                let _ = self.group.set(x);
                [y, z.strip_suffix(".git").unwrap_or(z)]
            }
        })
    }

    fn fetch_rev(&self, [owner, repo]: &[&str; 2]) -> Result<String> {
        let host = self.host.unwrap_or("gitlab.com");

        let mut url = format!("https://{host}/api/v4/projects/");
        if let Some(group) = self.group.get() {
            url.push_str(group);
            url.push_str("%2F");
        }
        write!(url, "{owner}%2F{repo}/repository/commits?per_page=1")?;

        let [Commit { id }] = ureq::get(&url)
            .call()?
            .into_json::<[_; 1]>()
            .with_context(|| {
                let mut msg = format!("no commits found for https://{host}/");
                if let Some(group) = self.group.get() {
                    msg.push_str(group);
                    msg.push('/');
                }
                msg.push_str(owner);
                msg.push('/');
                msg.push_str(repo);
                msg
            })?;

        Ok(id)
    }
}

impl<'a> SimpleGitFetcher<'a, 2> for FetchFromGitLab<'a> {
    fn get_flake_ref(&self, [owner, repo]: &[&str; 2], rev: &str) -> String {
        let mut flake_ref = String::from("gitlab:");
        if let Some(group) = self.group.get() {
            flake_ref.push_str(group);
            flake_ref.push_str("%2F");
        }
        flake_ref.push_str(owner);
        flake_ref.push('/');
        flake_ref.push_str(repo);
        flake_ref.push('/');
        flake_ref.push_str(rev);
        if let Some(host) = self.host {
            flake_ref.push_str("?host=");
            flake_ref.push_str(host);
        }
        flake_ref
    }

    fn get_repo_url(&self, [owner, repo]: &[&str; 2]) -> String {
        let mut flake_ref = String::from("git+https://");
        flake_ref.push_str(self.host.unwrap_or("gitlab.com"));
        flake_ref.push('/');
        if let Some(group) = self.group.get() {
            flake_ref.push_str(group);
            flake_ref.push('/');
        }
        flake_ref.push_str(owner);
        flake_ref.push('/');
        flake_ref.push_str(repo);
        flake_ref
    }
}
