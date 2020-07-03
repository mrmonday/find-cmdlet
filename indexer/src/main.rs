use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgGroup};

mod cmdlet;
mod error;
mod indexer;
mod markdown;
mod psgallery;

use indexer::Indexer;

fn main() -> anyhow::Result<()> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("markdown-directories")
                .short("m")
                .long("markdown-directory")
                .help("Directory containing markdown files to index")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("psgallery-directories")
                .short("p")
                .long("psgallery-directory")
                .help("Directory containing powershell gallery module data to index")
                .takes_value(true)
                .multiple(true),
        )
        .group(
            ArgGroup::with_name("input-directories")
                .required(true)
                .multiple(true)
                .args(&["psgallery-directories", "markdown-directories"]),
        )
        .arg(
            Arg::with_name("index-directory")
                .short("i")
                .long("index-directory")
                .help("Directory to produce the index in")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    pretty_env_logger::formatted_timed_builder()
        .filter_module("find_cmdlet", log::LevelFilter::Trace)
        .init();

    let index_directory = matches
        .value_of("index-directory")
        .expect("Index directory is a required parameter");
    let mut indexer = Indexer::new(index_directory)?;

    let markdown_directories = matches.values_of("markdown-directories");
    if let Some(markdown_directories) = markdown_directories {
        markdown::process_directories(&indexer, markdown_directories)?;
    }

    let psgallery_directories = matches.values_of("psgallery-directories");
    if let Some(psgallery_directories) = psgallery_directories {
        psgallery::process_directories(&indexer, psgallery_directories)?;
    }

    log::info!("Committing index");
    indexer.commit()?;

    Ok(())
}
