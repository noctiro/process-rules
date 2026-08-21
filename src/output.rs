use std::fs;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, tempdir_in};

use crate::application::SCHEMA_VERSION;
use crate::catalog::Catalog;
use crate::error::Error;
use crate::platform::{PlatformSelection, ProcessName};

/// Render exact process names as a Mihomo classical rule provider.
#[must_use]
pub fn render_mihomo(names: &[ProcessName]) -> String {
    let mut content = String::new();
    for name in names {
        content.push_str("PROCESS-NAME,");
        content.push_str(name.as_str());
        content.push('\n');
    }
    content
}

/// One generated file recorded in `manifest.json`.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ArtifactManifest {
    /// Slash-separated path relative to the output directory.
    pub path: String,
    /// Concrete platform or `all`.
    pub platform: String,
    /// Derived collection identifier.
    pub collection: String,
    /// Number of exact process rules.
    pub entries: usize,
    /// Lowercase SHA-256 digest of the artifact bytes.
    pub sha256: String,
}

/// Reproducible description of a complete output tree.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct BuildManifest {
    /// Generator package name.
    pub generator: String,
    /// Generator package version.
    pub generator_version: String,
    /// Input schema version.
    pub schema_version: u32,
    /// License applying to code, sources, and generated output.
    pub license: String,
    /// Compile-time source revision, or `unknown` for local builds.
    pub source_commit: String,
    /// Number of validated source applications.
    pub applications: usize,
    /// Every emitted provider file in stable order.
    pub artifacts: Vec<ArtifactManifest>,
}

/// Build every platform and collection into a directory.
///
/// The complete tree is staged next to the destination and installed with one rename.
///
/// # Errors
///
/// Returns an error if the destination is non-empty or unsafe, an artifact cannot be
/// written, the manifest cannot be serialized, or the staged directory cannot be installed.
pub fn build_all(catalog: &Catalog, out_dir: &Path) -> Result<BuildManifest, Error> {
    if out_dir.file_name().is_none() {
        return Err(Error::invalid_output(
            out_dir,
            "a named output directory is required",
        ));
    }
    let parent = usable_parent(out_dir);
    fs::create_dir_all(parent).map_err(|error| Error::io("create output parent", parent, error))?;
    prepare_directory_target(out_dir)?;

    let staging =
        tempdir_in(parent).map_err(|error| Error::io("create staging directory", parent, error))?;

    let collections = catalog.collections();
    let mut artifacts = Vec::with_capacity(PlatformSelection::ALL.len() * collections.len());
    let mihomo_dir = staging.path().join("mihomo");

    for platform in PlatformSelection::ALL {
        let platform_name = platform.as_str();
        let platform_dir = mihomo_dir.join(platform_name);
        fs::create_dir_all(&platform_dir)
            .map_err(|error| Error::io("create artifact directory", &platform_dir, error))?;

        for collection in &collections {
            let names = catalog.select_available(platform, collection);
            let rendered = render_mihomo(&names);
            let collection_name = collection.name();
            let relative_path = format!("mihomo/{platform_name}/{collection_name}.list");
            let destination = platform_dir.join(format!("{collection_name}.list"));
            fs::write(&destination, rendered.as_bytes())
                .map_err(|error| Error::io("write artifact", &destination, error))?;
            artifacts.push(ArtifactManifest {
                path: relative_path,
                platform: platform_name.to_owned(),
                collection: collection_name,
                entries: names.len(),
                sha256: sha256_hex(rendered.as_bytes()),
            });
        }
    }

    let manifest = BuildManifest {
        generator: env!("CARGO_PKG_NAME").to_owned(),
        generator_version: env!("CARGO_PKG_VERSION").to_owned(),
        schema_version: SCHEMA_VERSION,
        license: "GPL-3.0-or-later".to_owned(),
        source_commit: option_env!("PROCESS_RULES_SOURCE_COMMIT")
            .unwrap_or("unknown")
            .to_owned(),
        applications: catalog.application_count(),
        artifacts,
    };
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    manifest_bytes.push(b'\n');
    let manifest_path = staging.path().join("manifest.json");
    fs::write(&manifest_path, manifest_bytes)
        .map_err(|error| Error::io("write build manifest", &manifest_path, error))?;

    fs::rename(staging.path(), out_dir)
        .map_err(|error| Error::io("install output directory", out_dir, error))?;
    Ok(manifest)
}

/// Atomically replace one regular output file, creating its parent as needed.
///
/// # Errors
///
/// Returns an error if the target is not a regular file location, its parent
/// cannot be created, or the temporary file cannot be written and persisted.
pub fn write_atomic_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if path.file_name().is_none() {
        return Err(Error::invalid_output(
            path,
            "a regular filename is required",
        ));
    }
    if let Some(metadata) = existing_metadata(path)? {
        if metadata.file_type().is_symlink() {
            return Err(Error::invalid_output(
                path,
                "refusing to replace a symbolic link",
            ));
        }
        if metadata.is_dir() {
            return Err(Error::invalid_output(path, "destination is a directory"));
        }
    }

    let parent = usable_parent(path);
    fs::create_dir_all(parent).map_err(|error| Error::io("create output parent", parent, error))?;
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| Error::io("create temporary output", parent, error))?;
    temporary
        .write_all(bytes)
        .map_err(|error| Error::io("write temporary output", temporary.path(), error))?;
    temporary
        .persist(path)
        .map_err(|error| Error::io("replace output file", path, error.error))?;
    Ok(())
}

fn prepare_directory_target(out_dir: &Path) -> Result<(), Error> {
    if let Some(metadata) = existing_metadata(out_dir)? {
        if metadata.file_type().is_symlink() {
            return Err(Error::invalid_output(
                out_dir,
                "refusing to replace a symbolic link",
            ));
        }
        if !metadata.is_dir() {
            return Err(Error::invalid_output(
                out_dir,
                "destination exists and is not a directory",
            ));
        }
        let mut entries = fs::read_dir(out_dir)
            .map_err(|error| Error::io("inspect output directory", out_dir, error))?;
        if entries
            .next()
            .transpose()
            .map_err(|error| Error::io("inspect output directory", out_dir, error))?
            .is_some()
        {
            return Err(Error::invalid_output(
                out_dir,
                "destination directory must be empty",
            ));
        }
        fs::remove_dir(out_dir)
            .map_err(|error| Error::io("remove empty output directory", out_dir, error))?;
    }
    Ok(())
}

fn existing_metadata(path: &Path) -> Result<Option<fs::Metadata>, Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::io("inspect output path", path, error)),
    }
}

fn usable_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::{TestResult, load_catalog};

    #[test]
    fn renders_classical_text() -> TestResult {
        let names = ["Alpha.exe", "it's-fine"]
            .into_iter()
            .map(ProcessName::parse)
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            render_mihomo(&names),
            "PROCESS-NAME,Alpha.exe\nPROCESS-NAME,it's-fine\n"
        );
        Ok(())
    }

    #[test]
    fn empty_collection_is_an_empty_list() {
        assert!(render_mihomo(&[]).is_empty());
    }

    #[test]
    fn complete_build_is_reproducible() -> TestResult {
        let directory = tempdir()?;
        let rules = directory.path().join("rules");
        fs::create_dir(&rules)?;
        fs::write(
            rules.join("other.toml"),
            "version = 1\n\n[demo]\nregions = [\"cn-mainland\"]\nlinux = [\"demo\"]\n",
        )?;
        let catalog = load_catalog(&rules, false).into_catalog()?;
        let first_output = directory.path().join("first");
        let second_output = directory.path().join("second");

        let first = build_all(&catalog, &first_output)?;
        let second = build_all(&catalog, &second_output)?;
        assert!(
            first_output
                .join("mihomo/linux/only-cn-mainland.list")
                .exists()
        );
        assert_eq!(first, second);
        assert_eq!(
            fs::read(first_output.join("manifest.json"))?,
            fs::read(second_output.join("manifest.json"))?
        );
        Ok(())
    }

    #[test]
    fn complete_build_refuses_a_nonempty_directory() -> TestResult {
        let directory = tempdir()?;
        let rules = directory.path().join("rules");
        fs::create_dir(&rules)?;
        fs::write(rules.join("other.toml"), "version = 1\n")?;
        let catalog = load_catalog(&rules, false).into_catalog()?;
        let output = directory.path().join("documents");
        fs::create_dir(&output)?;
        fs::write(output.join("important.txt"), "keep me")?;

        assert!(matches!(
            build_all(&catalog, &output),
            Err(Error::InvalidOutput { .. })
        ));
        assert_eq!(fs::read_to_string(output.join("important.txt"))?, "keep me");
        Ok(())
    }
}
