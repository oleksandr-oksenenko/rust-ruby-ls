use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result, Context};

use itertools::Itertools;

use crate::ruby_env_provider::RubyEnvProvider;

const RAILS_ROOT_PATHS: &[&str] = &["db", "spec"];

const AUTOLOAD_PATHS_CMD: &str = "rails runner 'puts ActiveSupport::Dependencies.autoload_paths'";

pub struct RubyFilenameConverter {
    root_path: PathBuf,
    autoload_paths: Vec<String>
}

impl RubyFilenameConverter {
    pub fn new(root_path: PathBuf, ruby_env_provider: &RubyEnvProvider) -> Result<RubyFilenameConverter> {
        let output = ruby_env_provider.run_context_command(AUTOLOAD_PATHS_CMD)
            .with_context(|| "Failed to run rails runner command")?;
        let mut autoload_paths: Vec<String> = String::from_utf8(output)?.split('\n')
            .map(|s| s.to_string()).unique().collect();

        let mut other_paths = RAILS_ROOT_PATHS.iter().map(|s| s.to_string()).collect();

        autoload_paths.append(&mut other_paths);

        Ok(RubyFilenameConverter {
            root_path,
            autoload_paths
        })
    }

    pub fn path_to_scope(&self, path: &Path) -> Result<Vec<String>> {
        let local_path = path.strip_prefix(&self.root_path)?.with_extension("");

        let local_path = self.autoload_paths
            .iter()
            .find_map(|p| local_path.as_path().strip_prefix(p).ok())
            .unwrap_or(&local_path);

        let (sucesses, failures): (Vec<_>, Vec<_>) = local_path
                                   .iter()
                                   .map(|os_str| {
                                       os_str
                                           .to_str()
                                           .ok_or(Err(anyhow!("Couldn't convert from OsStr to str")))
                                   })
        .partition_result();

        if !failures.is_empty() {
            return failures.into_iter().next().unwrap();
        }

        let result = sucesses.into_iter().map(Self::name_to_scope).collect();

        Ok(result)
    }

    fn name_to_scope(name: &str) -> String {
        name.split('_').map(Self::capitalize).join("")
    }

    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod path_to_scope_tests {
        // use super::*;

        // #[test]
        // fn test_path_to_scope() {
        //     let root_path = Path::new("/a/b/c");
        //     let converter = RubyFilenameConverter::new(root_path);
        //     let path = Path::new("/a/b/c/module_one/module_two_three/class_four.rb");
        //     let expected_scope = vec!["ModuleOne", "ModuleTwoThree", "ClassFour"];

        //     let result = converter.path_to_scope(path);

        //     println!("{result:?}");

        //     assert!(result.is_ok());
        //     let result = result.unwrap();
        //     assert_eq!(result, expected_scope);
        // }
    }

    #[test]
    fn test_name_to_scope() {
        assert_eq!("ModuleOneTwoThree", RubyFilenameConverter::name_to_scope("module_one_two_three"));
    }

    #[test]
    fn test_capitalize() {
        assert_eq!("Module", RubyFilenameConverter::capitalize("module"));
        assert_eq!("", RubyFilenameConverter::capitalize(""));
        assert_eq!("123", RubyFilenameConverter::capitalize("123"));
    }
}
