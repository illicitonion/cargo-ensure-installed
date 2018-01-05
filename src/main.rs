extern crate getopts;
extern crate semver;
extern crate toml;

use getopts::Options;
use semver::{Version, VersionReq};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use toml::Value;

fn main() {
    match main_impl() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
    }
}

fn main_impl() -> Result<(), String> {
    let cargo_home =
        std::env::var("CARGO_HOME").expect("CARGO_HOME environment variable was not set");
    let crates_toml = PathBuf::from(cargo_home).join(".crates.toml");

    let mut flags = Options::new();
    flags.reqopt("p", "package", "Name of package to install", "rustfmt");
    flags.reqopt(
        "v",
        "version",
        "Version requirement to ensure is installed (accepts any valid semver)",
        "0.9.0",
    );
    let options = match flags.parse(&std::env::args().collect::<Vec<_>>()) {
        Ok(options) => options,
        Err(err) => return Err(err.to_string()),
    };
    let package = options.opt_str("package").unwrap();
    let raw_version = options.opt_str("version").unwrap();
    let want_version = match VersionReq::parse(&raw_version) {
        Ok(v) => v,
        Err(err) => {
            return Err(format!(
                "Invalid version specified '{:?}': {:?}",
                raw_version,
                err
            ))
        }
    };

    let contents = {
        if crates_toml.exists() {
            match read_file_to_string(&crates_toml) {
                Ok(s) => s,
                Err(err) => return Err(format!("Error reading {:?}: {:?}", crates_toml, err)),
            }
        } else {
            String::new()
        }
    };

    match should_install(&crates_toml, &contents, &package, &want_version) {
        Ok(install) => {
            if install {
                let status = Command::new("cargo")
                    .arg("install")
                    .arg("--force")
                    .arg("--vers")
                    .arg(&raw_version)
                    .arg(&package)
                    .status()
                    .unwrap();
                if !status.success() {
                    return Err("Error running cargo install".to_owned());
                }
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn should_install(
    crates_toml_path: &Path,
    crates_toml_contents: &str,
    package: &str,
    want_version: &VersionReq,
) -> Result<bool, String> {
    if crates_toml_contents.len() == 0 {
        return Ok(true);
    }

    let value = match crates_toml_contents.parse::<Value>() {
        Ok(v) => v,
        Err(err) => return Err(format!("Error parsing {:?}: {:?}", crates_toml_path, err)),
    };
    let v1 = match value.get("v1") {
        Some(v) => v,
        None => {
            return Err(format!(
                "Invalid .crates.toml file at {:?}: Missing section 'v1'.",
                crates_toml_path
            ))
        }
    };
    let table = match v1.as_table() {
        Some(t) => t,
        None => {
            return Err(format!(
                "Invalid .crates.toml file at {:?}: v1 was not a table.",
                crates_toml_path
            ))
        }
    };
    let installed = table.keys().find(
        |k| k.starts_with(&format!("{} ", package)),
    );
    match installed {
        Some(line) => {
            let parts: Vec<_> = line.split(" ").collect();
            let raw_version = parts.get(1).unwrap();
            let have_version = match Version::parse(raw_version) {
                Ok(v) => v,
                Err(err) => {
                    return Err(format!(
                        "Invalid crates.toml file at {:?}: {:?} could not be parsed as a version: \
{:?}",
                        crates_toml_path,
                        raw_version,
                        err
                    ))
                }
            };
            Ok(!want_version.matches(&have_version))
        }
        None => Ok(true),
    }
}

fn read_file_to_string(p: &Path) -> Result<String, std::io::Error> {
    let mut s = String::new();
    let mut f = File::open(&p)?;
    f.read_to_string(&mut s)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::should_install;
    use semver::VersionReq;
    use std::path::PathBuf;

    pub fn some_path() -> PathBuf {
        PathBuf::from("/path/to/.crates.toml")
    }

    #[test]
    pub fn no_contents() {
        test(true, "");
    }

    #[test]
    pub fn no_packages() {
        test(true, "[v1]");
    }

    #[test]
    pub fn absent_package() {
        test(true, r###"[v1]
"protobuf 1.4.2 (registry+https://github.com/rust-lang/crates.io-index)" = ["foo", "bar"]"###);
    }

    #[test]
    pub fn exact_match() {
        test(false, r###"[v1]
"rustfmt 0.9.0 (registry+https://github.com/rust-lang/crates.io-index)" = ["rustfmt"]"###);
    }

    #[test]
    pub fn have_newer_but_compatible() {
        test(false, r###"[v1]
"rustfmt 0.9.1 (registry+https://github.com/rust-lang/crates.io-index)" = ["rustfmt"]"###);
    }

    #[test]
    pub fn have_newer_but_incompatible() {
        test(true, r###"[v1]
"rustfmt 0.10.0 (registry+https://github.com/rust-lang/crates.io-index)" = ["rustfmt"]"###);
    }

    #[test]
    pub fn have_older() {
        test(true, r###"[v1]
"rustfmt 0.8.0 (registry+https://github.com/rust-lang/crates.io-index)" = ["rustfmt"]"###);
    }

    #[test]
    pub fn carat_satisfied() {
        let crates_toml_contents =
            r###"[v1]
"rustfmt 0.0.9 (registry+https://github.com/rust-lang/crates.io-index)" = ["rustfmt"]"###;

        assert_eq!(
            should_install(
                &some_path(),
                crates_toml_contents,
                "rustfmt",
                &VersionReq::parse("^0.0.9").unwrap(),
            ),
            Ok(false)
        )
    }

    #[test]
    pub fn carat_unsatisfied() {
        let crates_toml_contents =
            r###"[v1]
"rustfmt 0.0.10 (registry+https://github.com/rust-lang/crates.io-index)" = ["rustfmt"]"###;

        assert_eq!(
            should_install(
                &some_path(),
                crates_toml_contents,
                "rustfmt",
                &VersionReq::parse("^0.0.9").unwrap(),
            ),
            Ok(true)
        )
    }

    fn test(want: bool, crates_toml_contents: &str) {
        assert_eq!(
            should_install(
                &some_path(),
                crates_toml_contents,
                "rustfmt",
                &VersionReq::parse("0.9.0").unwrap(),
            ),
            Ok(want)
        )
    }
}
