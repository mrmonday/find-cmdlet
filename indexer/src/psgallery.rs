use anyhow::Context;
use rayon::prelude::*;
use std::fs::File;
use std::{
    io::{BufReader, Read},
    path,
};

use crate::cmdlet::Cmdlet;
use crate::error::FindCmdletError;
use crate::indexer::Indexer;
use path::{Path, PathBuf};

trait TryIndex {
    type Ret;

    fn try_index(&self, idx1: &str, idx2: &str) -> &Self::Ret;
    fn try_index4(&self, idx1: &str, idx2: &str, idx3: &str, idx4: &str) -> &Self::Ret;
}

fn is_empty_str_val(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) if s.trim().is_empty() => true,
        _ => false,
    }
}

fn to_null(val: &serde_json::Value) -> &serde_json::Value {
    match val {
        serde_json::Value::Null => &serde_json::Value::Null,
        serde_json::Value::String(s) if s.trim().is_empty() => &serde_json::Value::Null,
        _ => val,
    }
}

impl TryIndex for serde_json::Value {
    type Ret = serde_json::Value;
    fn try_index(&self, idx1: &str, idx2: &str) -> &Self::Ret {
        let res = &self[idx1];
        if is_empty_str_val(res) {
            return to_null(&self[idx2]);
        }

        to_null(res)
    }

    fn try_index4(&self, idx1: &str, idx2: &str, idx3: &str, idx4: &str) -> &Self::Ret {
        let res = &self[idx1];
        // Who wrote this monstrosity...
        if is_empty_str_val(res) {
            let res = &self[idx2];
            if is_empty_str_val(res) {
                let res = &self[idx3];
                if is_empty_str_val(res) {
                    return to_null(&self[idx4]);
                } else {
                    return to_null(res);
                }
            } else {
                return to_null(res);
            }
        }

        to_null(res)
    }
}

fn read_json<P: AsRef<path::Path>>(path: P) -> anyhow::Result<serde_json::Value> {
    let file = File::open(&path)
        .with_context(|| format!("could not open: {}", path.as_ref().to_string_lossy()))?;
    let mut buf_reader = BufReader::new(file);
    let mut bytes = Vec::with_capacity(2 * 1024 * 1024);
    buf_reader.read_to_end(&mut bytes).with_context(|| {
        format!(
            "failed to load json string from: {}",
            path.as_ref().to_string_lossy()
        )
    })?;

    let json: serde_json::Value = serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "failed to parse json from: {}",
            path.as_ref().to_string_lossy()
        )
    })?;

    Ok(json)
}

fn process_file_json(
    module_metadata: &ModuleMetaData,
    module_json: serde_json::Value,
    command_json: serde_json::Value,
    help_json: serde_json::Value,
) -> anyhow::Result<Cmdlet> {
    let name = help_json
        .try_index("name", "Name")
        .as_str()
        .or_else(|| {
            help_json
                .try_index("details", "Details")
                .try_index("name", "Name")
                .as_str()
        })
        .ok_or(FindCmdletError::MissingCmdletName)?
        .trim()
        .to_string();

    let url = command_json
        .try_index4("HelpUri", "helpUri", "Helpuri", "helpuri")
        .as_str()
        .or_else(|| {
            module_json
                .try_index4("ProjectUri", "projectUri", "Projecturi", "projecturi")
                .as_str()
        })
        .unwrap_or(&format!(
            "https://www.powershellgallery.com/packages/{}/{}",
            module_metadata.name, module_metadata.version
        ))
        .trim()
        .to_string();

    let mut tags = module_json
        .try_index("Tags", "tags")
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            let v = v.as_str()?;
            if v.trim() == "" {
                None
            } else {
                Some(v.to_string())
            }
        })
        .collect::<Vec<_>>();

    tags.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    tags.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    let synopsis = help_json
        .try_index("synopsis", "Synopsis")
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    let syntax = "".to_string(); // TODO build from parameters

    let description = help_json.try_index("description", "Description")[0]
        .try_index("text", "Text")
        .as_str()
        .or_else(|| {
            help_json
                .try_index("details", "Details")
                .try_index("description", "Description")[0]
                .try_index("text", "Text")
                .as_str()
        })
        .unwrap_or("")
        .trim()
        .to_string();

    let notes = "".to_string(); // TODO Maybe remarks?

    Ok(Cmdlet {
        module: module_metadata.name.clone(),
        module_version: module_metadata.version.clone(),
        name,
        url,
        tags,
        synopsis,
        syntax,
        description,
        notes,
    })
}

fn is_json_file(e: &walkdir::DirEntry) -> bool {
    let is_dir = e.file_type().is_dir();
    let is_json = e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".json");

    is_dir || is_json
}

#[derive(Clone)]
struct ModuleMetaData {
    name: String,
    version: String,
    docs_dir: PathBuf,
}

fn process_metadata_json<P: AsRef<path::Path>>(path: P) -> anyhow::Result<ModuleMetaData> {
    let json = read_json(&path)?;

    let name = json
        .try_index("Name", "name")
        .as_str()
        .ok_or(FindCmdletError::MissingModuleName)?
        .to_string();
    let version = json
        .try_index("Version", "version")
        .as_str()
        .ok_or(FindCmdletError::MissingModuleVersion)?
        .to_string();

    let docs_dir = path
        .as_ref()
        .parent()
        .expect("file cannot exist outside of directory")
        .parent()
        .expect("parent directory must exist")
        .join("docs");

    Ok(ModuleMetaData {
        name,
        version,
        docs_dir,
    })
}

struct MetaDataIterator {
    dir_walker: Box<dyn Iterator<Item = walkdir::Result<walkdir::DirEntry>>>,
}

impl MetaDataIterator {
    pub fn new<P: AsRef<path::Path>>(metadata_dir: P) -> Self {
        log::info!("metadata_dir: {:?}", metadata_dir.as_ref());
        let dir_walker = walkdir::WalkDir::new(metadata_dir)
            .into_iter()
            .filter_entry(&is_json_file);

        MetaDataIterator {
            dir_walker: Box::new(dir_walker),
        }
    }
}

impl Iterator for MetaDataIterator {
    type Item = anyhow::Result<ModuleMetaData>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.dir_walker.next();
            if let Some(Ok(ref de)) = &next {
                if de.file_type().is_dir() {
                    continue;
                }
            }

            return match next {
                Some(Ok(de)) => Some(process_metadata_json(de.path())),
                Some(Err(e)) => Some(Err(e.into())),
                None => None,
            };
        }
    }
}

struct PSGalleryCmdletFileIter {
    module_metadata: ModuleMetaData,
    module_path: path::PathBuf,
    help_dir: path::PathBuf,
    cmd_file_iter: Box<dyn Iterator<Item = walkdir::Result<walkdir::DirEntry>>>,
}

impl PSGalleryCmdletFileIter {
    pub fn new<P: AsRef<path::Path>>(
        module_metadata: ModuleMetaData,
        docs_root: P,
    ) -> anyhow::Result<Self> {
        let doc_dir = docs_root
            .as_ref()
            .join(&module_metadata.name)
            .join(&module_metadata.version);

        let module_path = doc_dir.join("mod.json");
        let cmd_dir = doc_dir.join("commands");
        let help_dir = doc_dir.join("help");

        if !module_path.exists() {
            return Err(FindCmdletError::MissingModJson(
                module_metadata.name.clone(),
                module_metadata.version,
                module_path.to_string_lossy().to_string(),
            )
            .into());
        }

        if !cmd_dir.exists() || !cmd_dir.is_dir() {
            return Err(FindCmdletError::MissingCmdDir(
                module_metadata.name.clone(),
                module_metadata.version,
                cmd_dir.to_string_lossy().to_string(),
            )
            .into());
        }

        if !help_dir.exists() || !help_dir.is_dir() {
            return Err(FindCmdletError::MissingHelpDir(
                module_metadata.name.clone(),
                module_metadata.version,
                help_dir.to_string_lossy().to_string(),
            )
            .into());
        }

        let commands = walkdir::WalkDir::new(cmd_dir)
            .into_iter()
            .filter_entry(&is_json_file);

        Ok(PSGalleryCmdletFileIter {
            module_metadata,
            module_path,
            help_dir,
            cmd_file_iter: Box::new(commands),
        })
    }
}

struct CmdletFiles {
    module_path: PathBuf,
    command_path: PathBuf,
    help_path: PathBuf,
}

impl Iterator for PSGalleryCmdletFileIter {
    //type Item = anyhow::Result<Vec<path::PathBuf>>;
    type Item = anyhow::Result<CmdletFiles>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.cmd_file_iter.next();
            if let Some(Ok(ref de)) = &next {
                if de.file_type().is_dir() {
                    continue;
                }
            }

            return match next {
                Some(Ok(command)) => {
                    let file_name = match command.path().file_name() {
                        Some(f) => f,
                        None => {
                            return Some(Err(FindCmdletError::MissingCommandFileName(
                                self.module_metadata.name.clone(),
                                self.module_metadata.name.clone(),
                                "(None)".to_string(),
                            )
                            .into()));
                        }
                    };
                    let help_path = self.help_dir.join(file_name);
                    if !help_path.exists() {
                        return Some(Err(FindCmdletError::MissingHelpText(
                            file_name.to_string_lossy().to_string(),
                            self.module_metadata.name.clone(),
                            self.module_metadata.name.clone(),
                            help_path.to_string_lossy().to_string(),
                        )
                        .into()));
                    }

                    Some(Ok(CmdletFiles {
                        module_path: self.module_path.to_path_buf(),
                        command_path: command.path().to_path_buf(),
                        help_path,
                    }))
                    // TODO Vec out of laziness - should be a proper struct
                    //Some(Ok(vec![
                    //    self.module_path.to_path_buf(),
                    //    command.path().to_path_buf(),
                    //    help_path,
                    //]))
                }
                Some(Err(e)) => Some(Err(e.into())),
                None => None,
            };
        }
    }
}

fn process_module_metadata(
    module_metadata: &ModuleMetaData,
    indexer: &Indexer,
) -> anyhow::Result<()> {
    log::info!(
        "Processing {} [{}]",
        module_metadata.name,
        module_metadata.version
    );

    let iter = PSGalleryCmdletFileIter::new(module_metadata.clone(), &module_metadata.docs_dir)?;
    //match iter {
    //    Ok(iter) => {
    iter.collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|files| match files {
            Ok(files) => {
                let ((module_json, command_json), help_json) = rayon::join(
                    || {
                        rayon::join(
                            || read_json(&files.module_path),
                            || read_json(&files.command_path),
                        )
                    },
                    || read_json(&files.help_path),
                );
                if let Err(e) = &module_json {
                    log::warn!("{:?}", e);
                }
                if let Err(e) = &command_json {
                    log::warn!("{:?}", e);
                }
                if let Err(e) = &help_json {
                    log::warn!("{:?}", e);
                }
                match (module_json, command_json, help_json) {
                    (Ok(m), Ok(c), Ok(h)) => Some((m, c, h)),
                    _ => None,
                }
                //let files: Vec<_> = files.into_par_iter().map(read_json).collect();
                //files
                //    .into_iter()
                //    .fold(
                //        None,
                //        |a: Option<anyhow::Result<Vec<serde_json::Value>>>, b| match b {
                //            Ok(f) => match a {
                //                Some(Ok(mut fs)) => {
                //                    fs.push(f);

                //                    Some(Ok(fs))
                //                }
                //                Some(Err(e)) => Some(Err(e)),
                //                None => Some(Ok(vec![f])),
                //            },
                //            Err(e) => Some(Err(e)),
                //        },
                //    )
                //    .expect("should always have files...")
            }
            Err(e) => {
                log::warn!("{:?}", e);

                None
            }
        })
        .for_each(|(module_json, command_json, help_json)| {
            let cmdlet = process_file_json(&module_metadata, module_json, command_json, help_json);
            match cmdlet {
                Ok(cmdlet) => indexer.update(&cmdlet),
                Err(e) => log::warn!("{:?}", e),
            }
        });
    //.for_each(|cmdlet_files| match cmdlet_files {
    //    Ok(mut cmdlet_files) => {
    //        let help_json = cmdlet_files.pop().unwrap();
    //        let command_json = cmdlet_files.pop().unwrap();
    //        let module_json = cmdlet_files.pop().unwrap();
    //        let cmdlet = process_file_json(
    //            &module_metadata,
    //            module_json,
    //            command_json,
    //            help_json,
    //        );
    //        match cmdlet {
    //            Ok(cmdlet) => indexer.update(&cmdlet),
    //            Err(e) => log::warn!("{:?}", e),
    //        }
    //    }
    //    Err(e) => log::warn!("{:?}", e),
    //});
    //    }
    //    Err(e) => {
    //        log::warn!("{:?}", e);
    //    }
    //}

    Ok(())
}

pub fn process_directories<I>(indexer: &Indexer, directories: I) -> anyhow::Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    log::info!("Processing PowerShell Gallery modules");

    directories
        .into_iter()
        .map(|d| MetaDataIterator::new(d.as_ref().join("metadata")))
        .flatten()
        .collect::<Vec<_>>()
        .into_par_iter()
        .for_each(|module_metadata| match module_metadata {
            Ok(module_metadata) => {
                if let Err(e) = process_module_metadata(&module_metadata, &indexer) {
                    log::warn!("{:?}", e);
                }
            }
            Err(e) => log::warn!("{:?}", e),
        });

    Ok(())
}
