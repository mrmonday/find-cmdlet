use crate::cmdlet::Cmdlet;
use crate::error::FindCmdletError;
use find_cmdlet_index::pascal_splitter;
use std::{fs::DirBuilder, path::Path};
use tantivy::doc;

pub struct Indexer {
    writer: tantivy::IndexWriter,

    module_field: tantivy::schema::Field,
    module_version_field: tantivy::schema::Field,
    name_field: tantivy::schema::Field,
    url_field: tantivy::schema::Field,
    tags_field: tantivy::schema::Field,
    synopsis_field: tantivy::schema::Field,
    syntax_field: tantivy::schema::Field,
    description_field: tantivy::schema::Field,
    notes_field: tantivy::schema::Field,
}

impl Indexer {
    pub fn new(directory: impl AsRef<Path>) -> anyhow::Result<Indexer> {
        let mut schema_builder = tantivy::schema::SchemaBuilder::default();

        let indexed_text_options = tantivy::schema::TextOptions::default().set_indexing_options(
            tantivy::schema::TextFieldIndexing::default()
                .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions)
                .set_tokenizer("en_stem"),
        );
        let stored_text_options = indexed_text_options.clone().set_stored();

        let cmdlet_name_options = tantivy::schema::TextOptions::default()
            .set_stored()
            .set_indexing_options(
                tantivy::schema::TextFieldIndexing::default()
                    .set_index_option(tantivy::schema::IndexRecordOption::Basic)
                    .set_tokenizer("pascal"),
            );

        let module_name = schema_builder.add_text_field(
            "module_name",
            tantivy::schema::TEXT | tantivy::schema::STORED,
        );
        let module_version =
            schema_builder.add_text_field("module_version", tantivy::schema::STORED);
        let name = schema_builder.add_text_field("name", cmdlet_name_options);
        let url = schema_builder.add_text_field("url", tantivy::schema::STORED);
        let tags =
            schema_builder.add_text_field("tags", tantivy::schema::TEXT | tantivy::schema::STORED);
        let synopsis = schema_builder.add_text_field("synopsis", stored_text_options);
        let syntax = schema_builder
            .add_text_field("syntax", tantivy::schema::TEXT | tantivy::schema::STORED);
        let description =
            schema_builder.add_text_field("description", indexed_text_options.clone());
        let notes = schema_builder.add_text_field("notes", indexed_text_options);

        let schema = schema_builder.build();
        DirBuilder::new().recursive(true).create(&directory)?;
        let index = tantivy::Index::create_in_dir(&directory, schema)
            .map_err(FindCmdletError::TantivyError)?;

        pascal_splitter::register(&index);

        let index_writer = index
            .writer(4_000_000_000)
            .map_err(FindCmdletError::TantivyError)?;

        Ok(Indexer {
            writer: index_writer,

            module_field: module_name,
            module_version_field: module_version,
            name_field: name,
            url_field: url,
            tags_field: tags,
            synopsis_field: synopsis,
            syntax_field: syntax,
            description_field: description,
            notes_field: notes,
        })
    }

    pub fn update(&self, cmdlet: &Cmdlet) {
        self.writer.add_document(doc!(
            self.module_field => cmdlet.module.clone(),
            self.module_version_field => cmdlet.module_version.clone(),
            self.name_field => cmdlet.name.clone(),
            self.url_field => cmdlet.url.clone(),
            self.tags_field => cmdlet.tags.join(" "),
            self.synopsis_field => cmdlet.synopsis.clone(),
            self.syntax_field => cmdlet.syntax.clone(),
            self.description_field => cmdlet.description.clone(),
            self.notes_field => cmdlet.notes.clone(),
        ));
    }

    pub fn commit(&mut self) -> anyhow::Result<u64> {
        self.writer
            .commit()
            .map_err(|e| FindCmdletError::TantivyError(e).into())
    }
}
