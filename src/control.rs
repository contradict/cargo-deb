use std::io::{self, Write};
use std::path::PathBuf;
use config::Config;
use md5::Digest;
use md5;
use file;
use std::collections::HashMap;
use error::*;
use std::os::unix::ffi::OsStrExt;
use archive::Archive;

/// Generates the uncompressed control.tar archive
pub fn generate_archive(options: &Config, time: u64, asset_hashes: HashMap<PathBuf, Digest>) -> CDResult<Vec<u8>> {
    let mut archive = Archive::new(time);
    initialize_control(&mut archive)?;
    generate_md5sums(&mut archive, options, asset_hashes)?;
    generate_control(&mut archive, options)?;
    if let Some(ref files) = options.conf_files {
        generate_conf_files(&mut archive, files)?;
    }
    generate_scripts(&mut archive, options)?;
    Ok(archive.into_inner()?)
}

/// Creates the initial hidden directory where all the files are stored.
fn initialize_control(archive: &mut Archive) -> io::Result<()> {
    if ::TAR_REJECTS_CUR_DIR {
        return Ok(());
    }
    archive.directory("./")
}

/// Append all files that reside in the `maintainer_scripts` path to the archive
fn generate_scripts(archive: &mut Archive, option: &Config) -> io::Result<()> {
    if let Some(ref maintainer_scripts) = option.maintainer_scripts {
        for name in &["preinst", "postinst", "prerm", "postrm"] {
            if let Ok(script) = file::get(maintainer_scripts.join(name)) {
                archive.file(name, &script, 0o755)?;
            }
        }
    }
    Ok(())
}

/// Creates the md5sums file which contains a list of all contained files and the md5sums of each.
fn generate_md5sums(archive: &mut Archive, options: &Config, asset_hashes: HashMap<PathBuf, md5::Digest>) -> CDResult<()> {
    let mut md5sums: Vec<u8> = Vec::new();

    // Collect md5sums from each asset in the archive.
    for asset in &options.assets {
        write!(md5sums, "{:x}", asset_hashes[&asset.source_file])?;
        md5sums.write(b"  ")?;

        md5sums.write(asset.target_path.as_os_str().as_bytes())?;
        md5sums.write(&[b'\n'])?;
    }

    // Write the data to the archive
    archive.file("./md5sums", &md5sums, 0o644)?;
    Ok(())
}

/// Generates the control file that obtains all the important information about the package.
fn generate_control(archive: &mut Archive, options: &Config) -> CDResult<()> {
    // Create and return the handle to the control file with write access.
    let mut control: Vec<u8> = Vec::with_capacity(1024);

    // Write all of the lines required by the control file.
    write!(&mut control, "Package: {}\n", options.name)?;
    write!(&mut control, "Version: {}\n", options.version)?;
    write!(&mut control, "Architecture: {}\n", options.architecture)?;
    if let Some(ref repo) = options.repository {
        if repo.starts_with("http") {
            write!(&mut control, "Vcs-Browser: {}\n", repo)?;
        }
        if let Some(kind) = options.repository_type() {
            write!(&mut control, "Vcs-{}: {}\n", kind, repo)?;
        }
    }
    if let Some(ref homepage) = options.homepage.as_ref().or(options.documentation.as_ref()) {
        write!(&mut control, "Homepage: {}\n", homepage)?;
    }
    if let Some(ref section) = options.section {
        write!(&mut control, "Section: {}\n", section)?;
    }
    write!(&mut control, "Priority: {}\n", options.priority)?;
    control.write(b"Standards-Version: 3.9.4\n")?;
    write!(&mut control, "Maintainer: {}\n", options.maintainer)?;
    write!(&mut control, "Depends: {}\n", options.get_dependencies()?)?;
    write!(&mut control, "Description: {}\n", options.description.replace('\n',"  "))?;

    // Write each of the lines that were collected from the extended_description to the file.
    for line in &options.extended_description {
        write!(&mut control, " {}\n", line)?;
    }
    control.push(10);

    // Add the control file to the tar archive.
    archive.file("./control", &control, 0o644)?;
    Ok(())
}

/// If configuration files are required, the conffiles file will be created.
fn generate_conf_files(archive: &mut Archive, files: &str) -> io::Result<()> {
    let mut data = Vec::new();
    data.write(files.as_bytes())?;
    data.push(b'\n');
    archive.file("./conffiles", &data, 0o644)
}
