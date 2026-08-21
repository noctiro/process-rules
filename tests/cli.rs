//! End-to-end tests for the `process-rules` command-line interface.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn fixture() -> TestResult<TempDir> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("apps.toml"),
        r#"version = 1

[china]
display_name = "China"
regions = ["cn-mainland"]
windows = ["China.exe"]
linux = ["china"]

[mixed]
regions = ["cn-mainland", "jp"]
windows = ["Mixed.exe"]

[foreign]
regions = ["us"]
macos = ["Foreign"]

[global]
regions = "any"
android = ["org.example.global"]
"#,
    )?;
    Ok(directory)
}

fn run(rules: &Path, arguments: &[&str]) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_process-rules"))
        .arg("--rules-dir")
        .arg(rules)
        .args(arguments)
        .output()?)
}

#[test]
fn validate_build_and_explain_work_end_to_end() -> TestResult {
    let rules = fixture()?;
    let validation = run(rules.path(), &["validate"])?;
    assert!(validation.status.success());
    assert_eq!(
        String::from_utf8(validation.stdout)?,
        "valid: 4 application(s), 0 warning(s)\n"
    );
    assert!(validation.stderr.is_empty());

    let build = run(
        rules.path(),
        &[
            "build",
            "--platform",
            "windows",
            "--collection",
            "contain-cn-mainland",
        ],
    )?;
    assert!(build.status.success());
    assert_eq!(
        String::from_utf8(build.stdout)?,
        "PROCESS-NAME,China.exe\nPROCESS-NAME,Mixed.exe\n"
    );

    let explain = run(
        rules.path(),
        &["explain", "--platform", "windows", "--process", "china.EXE"],
    )?;
    assert!(explain.status.success());
    let explanation = String::from_utf8(explain.stdout)?;
    assert!(explanation.contains("application: china\n"));
    assert!(explanation.contains("regions: cn-mainland\n"));
    assert!(explanation.contains(
        "collections: only-cn-mainland, contain-cn-mainland, not-contain-jp, not-contain-us\n"
    ));
    Ok(())
}

#[test]
fn json_validation_diagnostics_are_machine_readable() -> TestResult {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("invalid.toml"),
        r#"version = 1

[first]
regions = ["jp"]
windows = ["Same.exe"]

[second]
regions = ["us"]
windows = ["same.EXE"]
"#,
    )?;

    let output = run(
        directory.path(),
        &["--diagnostic-format", "json", "validate"],
    )?;
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let diagnostics: Value = serde_json::from_slice(&output.stderr)?;
    let codes = diagnostics
        .as_array()
        .ok_or("expected diagnostic JSON array")?
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"duplicate-process-name"));
    assert!(codes.contains(&"conflicting-all-platform-regions"));
    Ok(())
}

#[test]
fn build_all_emits_complete_list_tree() -> TestResult {
    let rules = fixture()?;
    let output_directory = rules.path().join("dist");
    let output_path = output_directory
        .to_str()
        .ok_or("temporary fixture path is not valid UTF-8")?;
    let output = run(rules.path(), &["build-all", "--out-dir", output_path])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output_directory
            .join("mihomo/windows/only-cn-mainland.list")
            .exists()
    );
    assert!(
        output_directory
            .join("mihomo/all/not-contain-us.list")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(output_directory.join("mihomo/windows/only-cn-mainland.list"))?,
        "PROCESS-NAME,China.exe\n"
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(output_directory.join("manifest.json"))?)?;
    assert_eq!(manifest["license"], "GPL-3.0-or-later");
    assert_eq!(manifest["applications"], 4);
    let artifacts = manifest["artifacts"]
        .as_array()
        .ok_or("expected manifest artifact array")?;
    assert_eq!(artifacts.len(), 50);
    assert!(artifacts.iter().all(|artifact| {
        artifact["path"].as_str().is_some_and(|path| {
            Path::new(path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("list"))
        })
    }));
    Ok(())
}

#[test]
fn invalid_collection_and_missing_explanation_have_stable_exit_code() -> TestResult {
    let rules = fixture()?;
    let invalid_collection = run(
        rules.path(),
        &["build", "--platform", "linux", "--collection", "cn-ish"],
    )?;
    assert_eq!(invalid_collection.status.code(), Some(4));

    let noncanonical_cn_alias = run(
        rules.path(),
        &["build", "--platform", "linux", "--collection", "only-cn"],
    )?;
    assert_eq!(noncanonical_cn_alias.status.code(), Some(4));

    let unavailable_region = run(
        rules.path(),
        &["build", "--platform", "linux", "--collection", "only-ad"],
    )?;
    assert_eq!(unavailable_region.status.code(), Some(4));
    assert!(
        String::from_utf8_lossy(&unavailable_region.stderr)
            .contains("region does not occur in the catalog")
    );

    let not_found = run(
        rules.path(),
        &["explain", "--platform", "all", "--process", "missing"],
    )?;
    assert_eq!(not_found.status.code(), Some(4));
    Ok(())
}

#[test]
fn publishing_policy_rejects_an_empty_catalog_with_json_diagnostics() -> TestResult {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("other.toml"), "version = 1\n")?;

    let output = run(
        directory.path(),
        &[
            "--diagnostic-format",
            "json",
            "validate",
            "--require-non-empty",
        ],
    )?;
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let diagnostics: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(diagnostics[0]["code"], "empty-catalog");
    Ok(())
}
