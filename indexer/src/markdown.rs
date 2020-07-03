use crate::cmdlet::Cmdlet;
use crate::error::FindCmdletError;
use crate::indexer::Indexer;
use path::Path;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path;

#[derive(PartialEq, Eq)]
enum MDState {
    FindHeading,
    StoreHeading,
    StoreText,
}

fn process_file_md<P: AsRef<path::Path>>(path: P) -> anyhow::Result<Cmdlet> {
    let arena = comrak::Arena::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut text = String::new();
    buf_reader.read_to_string(&mut text)?;

    // comrak doesn't support yaml metadata, so hack around it...
    let matches = text.match_indices("---");
    let mut found = 0;
    let mut markdown_start_idx = 0;
    let mut yaml_start_idx = 0;
    let mut yaml_end_idx = 0;
    for (idx, _) in matches {
        if found == 0 && idx != 0 {
            return Err(FindCmdletError::MissingMetaDataStart.into());
        }
        let is_crlf = text.get(idx + 3..idx + 5) == Some("\r\n");
        let is_lf = text.get(idx + 3..idx + 4) == Some("\n");
        if is_crlf || is_lf {
            found += 1;
            let add = if is_crlf { 2 } else { 1 };
            if found == 2 {
                yaml_end_idx = idx;
                markdown_start_idx = idx + 3 + add;
                break;
            } else {
                yaml_start_idx = idx + 3 + add;
            }
        }
    }
    if found != 2 {
        return Err(FindCmdletError::MissingMetaDataEnd.into());
    }

    let metadata = yaml_rust::YamlLoader::load_from_str(&text[yaml_start_idx..yaml_end_idx])?;
    if metadata.len() != 1 || metadata[0].as_hash().is_none() {
        return Err(FindCmdletError::UnexpectedMetaData.into());
    }
    let metadata = &metadata[0];
    //println!("{:?}", metadata);

    let doc = comrak::parse_document(
        &arena,
        &text[markdown_start_idx..],
        &comrak::ComrakOptions::default(),
    );

    //println!("{:?}", doc);
    let mut sections = Vec::new();
    let mut current_heading = String::new();
    let mut current_text = String::new();
    let mut state = MDState::FindHeading;
    iter_nodes(doc, &mut |node| {
        match node.data.borrow().value {
            comrak::nodes::NodeValue::Text(ref text) => {
                let text = String::from_utf8_lossy(text);
                if state == MDState::StoreHeading {
                    if let Some(parent) = node.parent() {
                        if let comrak::nodes::NodeValue::Heading(_) = parent.data.borrow().value {
                            current_heading += &text;
                            state = MDState::StoreText;
                        }
                    }
                } else if state == MDState::StoreText {
                    current_text += &text;
                }
            }
            comrak::nodes::NodeValue::Heading(comrak::nodes::NodeHeading { level, .. }) => {
                if level == 2 {
                    state = MDState::StoreHeading;
                    sections.push((current_heading.clone(), current_text.clone()));
                    current_heading = String::new();
                    current_text = String::new();
                }
                //println!("heading {:?}: {:?}", level, node.children().next());
            }
            _ => {} //ref dat => println!("other: {:?}", node),
        }
    });
    sections.push((current_heading, current_text));

    //println!("{:?}", sections);

    let name = metadata["title"]
        .as_str()
        .ok_or(FindCmdletError::MissingCmdletName)?
        .to_string();
    let url = metadata["online version"]
        .as_str()
        .ok_or(FindCmdletError::MissingCmdletUrl)?
        .to_string();
    let tags = metadata["keywords"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .map(|x| x.to_string())
        .collect();

    let mut synopsis = String::new();
    let mut syntax = String::new();
    let mut description = String::new();
    let mut notes = String::new();
    for section in sections {
        match section.0.to_ascii_uppercase().as_str() {
            "SYNOPSIS" => synopsis = section.1,
            "SYNTAX" => syntax = section.1,
            "DESCRIPTION" => description = section.1,
            "NOTES" => notes = section.1,
            _ => {}
        }
    }

    Ok(Cmdlet {
        module: "TODO".to_string(),
        module_version: "TODO".to_string(),
        name,
        url,
        tags,
        synopsis,
        syntax,
        description,
        notes,
    })
}

pub fn process_directories<I>(indexer: &Indexer, directories: I) -> anyhow::Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let dir_walker = directories
        .into_iter()
        .map(|d| {
            walkdir::WalkDir::new(d).into_iter().filter_entry(|e| {
                e.file_type().is_dir()
                    || (e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".md"))
            })
        })
        .flatten();

    for dir_entry in dir_walker.into_iter() {
        match dir_entry {
            Ok(de) => {
                if de.file_type().is_dir() {
                    continue;
                }
                match process_file_md(de.path()) {
                    Ok(cmdlet) => indexer.update(&cmdlet),
                    Err(e) => log::warn!("{:?}", e),
                }
            }
            Err(e) => log::warn!("{:?}", e),
        }
    }

    Ok(())
}

fn iter_nodes<'a, F>(node: &'a comrak::nodes::AstNode<'a>, f: &mut F)
where
    F: FnMut(&'a comrak::nodes::AstNode<'a>),
{
    f(node);
    for c in node.children() {
        iter_nodes(c, f);
    }
}
