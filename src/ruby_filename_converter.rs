use std::path::Path;

use anyhow::{anyhow, Result};

use itertools::Itertools;

const RAILS_ROOT_PATHS: &[&str] = &["app/models", "db", "lib", "spec"];

pub fn path_to_scope(root_path: &Path, path: &Path) -> Result<Vec<String>> {
    let local_path = path.strip_prefix(root_path)?.with_extension("");

    let local_path = RAILS_ROOT_PATHS
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

    let result = sucesses.into_iter().map(name_to_scope).collect();

    Ok(result)
}

fn name_to_scope(name: &str) -> String {
    name.split('_').map(capitalize).join("")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod path_to_scope_tests {
        use super::*;

        #[test]
        fn test_path_to_scope() {
            let root_path = Path::new("/a/b/c");
            let path = Path::new("/a/b/c/module_one/module_two_three/class_four.rb");
            let expected_scope = vec!["ModuleOne", "ModuleTwoThree", "ClassFour"];

            let result = path_to_scope(root_path, path);

            println!("{result:?}");

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!(result, expected_scope);
        }
    }

    #[test]
    fn test_name_to_scope() {
        assert_eq!("ModuleOneTwoThree", name_to_scope("module_one_two_three"));
    }

    #[test]
    fn test_capitalize() {
        assert_eq!("Module", capitalize("module"));
        assert_eq!("", capitalize(""));
        assert_eq!("123", capitalize("123"));
    }
}
