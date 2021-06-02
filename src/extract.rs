use execute::Execute;
use log::{error, trace};
use std::path::Path;
use std::process::Command;

pub fn get_archive_type(path: &Path) -> &str {
    match infer::get_from_path(path) {
        Ok(Some(kind)) => kind.mime_type(),
        _ => "",
    }
}

fn extract_zip(archive_path: &Path, outpath: &Path) -> Command {
    let mut command = Command::new("unzip");
    command.arg("-qq");
    command.arg("-o");
    command.arg(archive_path);
    command.arg("-d");
    command.arg(outpath);

    command
}

fn extract_rar(archive_path: &Path, outpath: &Path) -> Command {
    let mut command = Command::new("unrar");
    command.arg("x");
    command.arg("-o+");
    command.arg(archive_path);
    command.arg(outpath);

    command
}

fn extract_7z(archive_path: &Path, outpath: &Path) -> Command {
    let mut command = Command::new("7z");
    command.arg("x");
    command.arg("-aoa");
    command.arg(format!("-o{}", outpath.to_string_lossy()));
    command.arg(archive_path);

    command
}

pub fn extract(archive_path: &Path, outpath: &Path) -> Result<(), &'static str> {
    if !archive_path.exists() {
        return Err("Archive does not exist!");
    }

    let archive_type = get_archive_type(archive_path);

    let mut command = match archive_type {
        "application/zip" => extract_zip(archive_path, outpath),
        "application/vnd.rar" => extract_rar(archive_path, outpath),
        "application/x-7z-compressed" => extract_7z(archive_path, outpath),
        _ => {
            error!(
                "{}: {} is not a supported archve!",
                archive_path.to_string_lossy(),
                archive_type
            );
            return Err("Unsupported archive");
        }
    };

    trace!("{:?}", command);
    if let Some(cmd_output) = command.execute().unwrap() {
        if cmd_output != 0 {
            error!("Failed to extract archive: {}", archive_path.display());
            return Err("Failed to extract");
        }
    }

    Ok(())
}
