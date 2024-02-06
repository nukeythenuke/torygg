use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use anyhow::anyhow;
use log::info;
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::{config, modmanager};
use crate::util::find_case_insensitive_path;

pub(crate) type FomodCallback = fn(&InstallStep) -> Vec<&Plugin>;

#[derive(Debug)]
pub enum GroupType {
    SelectExactlyOne,
    SelectAny,
    SelectAll
}

impl FromStr for GroupType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SelectExactlyOne" => Ok(Self::SelectExactlyOne),
            "SelectAny" => Ok(Self::SelectAny),
            "SelectAll" => Ok(Self::SelectAll),
            _ => Err(anyhow!("unknown group type {s}"))
        }
    }
}

#[derive(Debug)]
pub enum FileOrFolder {
    File {
        source: PathBuf,
        destination: PathBuf
    },
    Folder {
        source: PathBuf,
        destination: PathBuf
    }
}

#[derive(Debug)]
pub struct Plugin {
    name: String,
    description: Option<String>,
    files: Option<Vec<FileOrFolder>>
}

impl Plugin {
    fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            files: None
        }
    }

    fn set_description(&mut self, desc: String) {
        self.description = Some(desc);
    }

    fn push_file(&mut self, file: FileOrFolder) {
        if let Some(files) = self.files.as_mut() {
            files.push(file);
        } else {
            self.files = Some(Vec::new());
            self.push_file(file);
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    #[must_use]
    pub fn files(&self) -> Option<&Vec<FileOrFolder>> {
        self.files.as_ref()
    }
}

#[derive(Debug)]
pub struct FileGroup {
    name: String,
    group_type: GroupType,
    plugins: Vec<Plugin>,
}

impl FileGroup {
    fn new(name: String, group_type: GroupType) -> Self {
        Self {
            name,
            group_type,
            plugins: Vec::new()
        }
    }

    fn push(&mut self, plugin: Plugin) {
        self.plugins.push(plugin);
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn group_type(&self) -> &GroupType {
        &self.group_type
    }

    #[must_use]
    pub fn plugins(&self) -> &Vec<Plugin> {
        &self.plugins
    }
}

#[derive(Debug)]
pub struct InstallStep {
    name: String,
    file_groups: Option<Vec<FileGroup>>
}

impl InstallStep {
    fn with_name(name: String) -> Self {
        Self {
            name,
            file_groups: None
        }
    }

    fn add_file_group(&mut self, group: FileGroup) {
        if let Some(groups) = self.file_groups.as_mut() {
            groups.push(group);
        } else {
            self.file_groups = Some(Vec::new());
            self.add_file_group(group);
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn file_groups(&self) -> Option<&Vec<FileGroup>> {
        self.file_groups.as_ref()
    }
}

fn get_install_steps(module_config: &Path) -> Result<Vec<InstallStep>, ToryggError> {
    let file = File::open(module_config)?;
    let file = BufReader::new(file);
    let parser = xml::EventReader::new(file);

    let mut install_steps = Vec::new();
    let mut install_step_builder = None;
    let mut file_group = None;
    let mut plugin = None;
    let mut is_desc = false;
    for e in parser {
        match e.map_err(|_| ToryggError::Other("parser error".to_owned()))? {
            xml::reader::XmlEvent::StartElement { name, attributes, ..} => {
                match name.to_string().as_ref() {
                    "installStep" => {
                        install_step_builder = Some(InstallStep::with_name(attributes.first().unwrap().value.clone()));
                    }
                    "group" => {
                        file_group = Some(FileGroup::new(attributes[0].value.clone(), GroupType::from_str(&attributes[1].value).unwrap()));
                    }
                    "plugin" => {
                        plugin = Some(Plugin::new(attributes.first().unwrap().value.clone()));
                    }
                    "description" => {
                        is_desc = true;
                    }
                    "file" => {
                        plugin.as_mut().unwrap().push_file(FileOrFolder::File {
                            source: typed_path::WindowsPathBuf::from_str(&attributes[0].value).unwrap().with_unix_encoding().to_str().unwrap().into(),
                            destination: typed_path::WindowsPathBuf::from_str(&attributes[1].value).unwrap().with_unix_encoding().to_str().unwrap().into()
                        });
                    }
                    "folder" => {
                        plugin.as_mut().unwrap().push_file(FileOrFolder::Folder {
                            source: typed_path::WindowsPathBuf::from_str(&attributes[0].value).unwrap().with_unix_encoding().to_str().unwrap().into(),
                            destination: typed_path::WindowsPathBuf::from_str(&attributes[1].value).unwrap().with_unix_encoding().to_str().unwrap().into()
                        });
                    }
                    _ => {}
                }
            }
            xml::reader::XmlEvent::EndElement { name } => {
                match name.to_string().as_ref() {
                    "installStep" => {
                        install_steps.push(install_step_builder.take().unwrap());
                    }
                    "group" => {
                        install_step_builder.as_mut().unwrap().add_file_group(file_group.take().unwrap());
                    }
                    "plugin" => {
                        file_group.as_mut().unwrap().push(plugin.take().unwrap());
                    }
                    "description" => {
                        is_desc = false;
                    }
                    _ => {}
                }
            }
            xml::reader::XmlEvent::Characters(chars) => {
                if is_desc {
                    plugin.as_mut().unwrap().set_description(chars);
                }
            }
            _ => {}
        }
    }

    Ok(install_steps)
}

pub(crate) fn fomod_install(mod_root: &Path, fomod_dir: &Path, name: &String, fomod_callback: FomodCallback) -> Result<(), ToryggError> {
    let entries = fs::read_dir(fomod_dir)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let mut module_config = None;
    for entry in entries {
        if unicase::eq(entry.file_name().to_string_lossy().as_ref(), "ModuleConfig.xml") {
            module_config = Some(entry.path());
            break;
        }
    };

    let Some(module_config) = module_config else {
        println!("no ModuleConfig.xml, doing regular install");
        fs::remove_dir_all(fomod_dir)?;
        return modmanager::install_all(mod_root, name);
    };

    let install_steps = get_install_steps(&module_config).unwrap();
    for step in &install_steps {
        info!("steps:\n{}", step.name());
    }

    let plugins = install_steps.iter().flat_map(fomod_callback).collect::<Vec<_>>();

    let install_path = config::mods_dir().maybe_create_child_directory(name)?;

    for plugin in plugins {
        let Some(files) = plugin.files() else {
            continue;
        };

        for file in files {
            match file {
                FileOrFolder::File { source, destination} => {
                    let from = mod_root.join(source);
                    let relative_path = find_case_insensitive_path(&install_path, destination);

                    for path in relative_path.ancestors().skip(1).collect::<Vec<_>>().iter().rev() {
                        let _ = install_path.maybe_create_child_directory(path)?;
                    }

                    let to = install_path.as_ref().join(&relative_path);

                    info!("{from:?} -> {to:?}");
                    fs::copy(from, to)?;
                },
                FileOrFolder::Folder { source, destination} => {
                    let entries = WalkDir::new(mod_root.join(source))
                        .min_depth(1).into_iter()
                        .filter_map(Result::ok);

                    for entry in entries {
                        let from = entry.path();
                        let relative_path = from.strip_prefix(mod_root.join(source)).unwrap();
                        let relative_path = destination.join(relative_path);
                        let relative_path = find_case_insensitive_path(&install_path, &relative_path);
                        let to = install_path.maybe_create_child_directory(destination)?.as_ref().join(relative_path);

                        info!("{from:?} -> {to:?}");

                        if from.is_dir() {
                            if !to.exists() {
                                fs::create_dir(to)?;
                            }
                        } else {
                            fs::copy(from, to)?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}