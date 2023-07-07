use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use log::info;

pub struct RubyEnvProvider {
    dir: PathBuf,
}

impl RubyEnvProvider {
    pub fn new(dir: &Path) -> RubyEnvProvider {
        RubyEnvProvider {
            dir: dir.to_path_buf(),
        }
    }

    pub fn stubs_dir(&self) -> Result<Option<PathBuf>> {
        let ruby_version = match self.ruby_version()? {
            None => return Ok(None),
            Some(version) => version,
        };

        let segments: Vec<&str> = ruby_version.split('.').collect();
        let major = segments[0];
        let minor = segments[1];

        // TODO: detect user dir
        // TODO: support other version managers?
        let path = "/Users/oleksandr.oksenenko/code/rust-ruby-ls/stubs/rubystubs".to_owned() + major + minor;

        Ok(Some(PathBuf::from(path)))
    }

    pub fn gems_dir(&self) -> Result<Option<PathBuf>> {
        let ruby_version = match self.ruby_version()? {
            None => return Ok(None),
            Some(version) => version,
        };

        // TODO: detect user dir
        // TODO: support other version managers?
        let path = "/Users/oleksandr.oksenenko/.rvm/gems/ruby-".to_owned() + &ruby_version;
        match self.gemset()? {
            None => Ok(Some(PathBuf::from(path))),
            Some(gemset) => Ok(Some(PathBuf::from(path + "@" + &gemset))),
        }
    }

    pub fn ruby_bin_dir(&self) -> Result<Option<PathBuf>> {
        let gems_dir = self.gems_dir()?;
        Ok(gems_dir.map(|gems_dir| gems_dir.join("bin/")))
    }

    pub fn ruby_path(&self) -> Result<PathBuf> {
        let ruby_version = self.ruby_version()?.ok_or(anyhow!("Failed to determine ruby version"))?;
        let path = "/Users/oleksandr.oksenenko/.rvm/rubies/".to_owned() + &ruby_version + "/bin/ruby";
        Ok(PathBuf::from(path))
    }

    pub fn run_context_command(&self, args: &str) -> Result<Vec<u8>> {
        let bundle_path = self.ruby_bin_dir().with_context(|| "Failed to find ruby bin dir")?.map(|d| d.join("bundle"));
        let ruby_path = self.ruby_path().with_context(|| "Failed to find ruby path")?;
        let cmd = bundle_path.unwrap_or(ruby_path);

        let mut command = Command::new(cmd);
        let command = command.arg("exec").arg(args);

        info!("Running command: {command:?}");

        match command.output() {
            Ok(o) => {
                if o.status.success() {
                    Ok(o.stdout)
                } else {
                    Err(anyhow!("Rails runner command failed with {:?} exit code", o.status.code()))
                }
            }

            Err(e) => Err(e).with_context(|| format!("Failed to run context command: {args}")),
        }
    }

    fn ruby_version(&self) -> Result<Option<String>> {
        let ruby_version_file = self.dir.join(".ruby-version");
        if ruby_version_file.exists() {
            Ok(Some(fs::read_to_string(ruby_version_file)?.trim().to_owned()))
        } else {
            Ok(None)
        }
    }

    fn gemset(&self) -> Result<Option<String>> {
        let gemset_file = self.dir.join(".ruby-gemset");
        if gemset_file.exists() {
            Ok(Some(fs::read_to_string(gemset_file)?.trim().to_owned()))
        } else {
            Ok(None)
        }
    }
}
