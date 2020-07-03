use actix::prelude::*;
use actix_files as fs;
use actix_web::{http, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Context;
use find_cmdlet_index::pascal_splitter;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path,
};

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(rename = "q")]
    query: String,
    #[serde(rename = "t")]
    ty: Option<String>,
}

#[derive(Serialize)]
pub struct CmdletResult {
    module_name: String,
    module_version: String,
    name: String,
    url: String,
    tags: Vec<String>,
    synopsis: String,
    syntax: String,
    score: f32,
}

#[derive(Debug)]
enum SearchError {
    Tantivy(tantivy::TantivyError),
    TantivyQuery(tantivy::query::QueryParserError),
    None,
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SearchError::Tantivy(te) => te.fmt(f),
            SearchError::TantivyQuery(te) => te.fmt(f),
            SearchError::None => f.write_str("none error"),
        }
    }
}

impl std::error::Error for SearchError {}

fn search_index(index: &tantivy::Index, query_str: &str) -> anyhow::Result<Vec<CmdletResult>> {
    let reader = index
        .reader()
        .map_err(SearchError::Tantivy)
        .context("could not get reader for index")?;
    let searcher = reader.searcher();
    let query_parser = tantivy::query::QueryParser::for_index(
        &index,
        index
            .schema()
            .fields()
            .filter(|f| f.1.is_indexed())
            .map(|f| f.0)
            .collect(),
    );
    let query = query_parser
        .parse_query(query_str)
        .or_else(|_| {
            let query_str: String = query_str
                .chars()
                .filter_map(|c| {
                    if c.is_ascii() {
                        if c.is_alphanumeric() {
                            Some(c.to_ascii_lowercase())
                        } else {
                            None
                        }
                    } else {
                        Some(c)
                    }
                })
                .collect();

            query_parser.parse_query(&query_str)
        })
        .map_err(SearchError::TantivyQuery)
        .with_context(|| format!("could not parse query string: {}", query_str))?;

    let docs = searcher
        .search(&query, &tantivy::collector::TopDocs::with_limit(30))
        .map_err(SearchError::Tantivy)
        .with_context(|| format!("searching failed for query: {}", query_str))?;

    // TODO should probably share this with indexer?
    let module_name = index
        .schema()
        .get_field("module_name")
        .context("could not find module name in index")?;
    let module_version = index
        .schema()
        .get_field("module_version")
        .context("could not find module version in index")?;
    let name = index
        .schema()
        .get_field("name")
        .context("could not find name in index")?;
    let url = index
        .schema()
        .get_field("url")
        .context("could not find url in index")?;
    let tags = index
        .schema()
        .get_field("tags")
        .context("could not find tags in index")?;
    let synopsis = index
        .schema()
        .get_field("synopsis")
        .context("could not find synopsis in index")?;
    let syntax = index
        .schema()
        .get_field("syntax")
        .context("could not find syntax in index")?;

    Ok(docs
        .into_iter()
        .map(|(score, doc_addr)| -> anyhow::Result<_> {
            let doc = searcher
                .doc(doc_addr)
                .map_err(SearchError::Tantivy)
                .with_context(|| format!("could not find document for query: {}", query_str))?;

            let module_name = doc
                .get_first(module_name)
                .ok_or(SearchError::None)
                .context("could not find module name")?
                .text()
                .ok_or(SearchError::None)
                .context("could not find module name text")?;
            let module_version = doc
                .get_first(module_version)
                .ok_or(SearchError::None)
                .context("could not find module version")?
                .text()
                .ok_or(SearchError::None)
                .context("could not find module version text")?;
            let name = doc
                .get_first(name)
                .ok_or(SearchError::None)
                .context("could not find cmdlet name")?
                .text()
                .ok_or(SearchError::None)
                .context("could not find cmdlet name text")?;
            let url = doc
                .get_first(url)
                .ok_or(SearchError::None)
                .context("could not find url")?
                .text()
                .ok_or(SearchError::None)
                .context("could not find url text")?;
            let tags = doc
                .get_first(tags)
                .ok_or(SearchError::None)
                .context("could not find tags")?
                .text()
                .ok_or(SearchError::None)
                .context("could not find tags text")?;
            let synopsis = doc
                .get_first(synopsis)
                .ok_or(SearchError::None)
                .context("could not find synopsis")?
                .text()
                .ok_or(SearchError::None)
                .context("could not find synopsis text")?;
            let syntax = doc
                .get_first(syntax)
                .ok_or(SearchError::None)
                .context("could not find syntax")?
                .text()
                .ok_or(SearchError::None)
                .context("could not find syntax text")?;

            Ok(CmdletResult {
                module_name: module_name.trim().to_string(),
                module_version: module_version.trim().to_string(),
                name: name.trim().to_string(),
                url: url.to_string(),
                tags: tags
                    .split(' ')
                    .filter(|t| !t.trim().is_empty())
                    .map(|s| s.trim().to_string())
                    .collect(),
                synopsis: synopsis.trim().to_string(),
                syntax: syntax.trim().to_string(),
                score,
            })
        })
        .flatten()
        .collect())
}

fn ise(error: anyhow::Error) -> actix_web::error::Error {
    log::warn!("{:?}", error);

    HttpResponse::InternalServerError().finish().into()
}

async fn search(
    state: web::Data<State>,
    request: HttpRequest,
) -> actix_web::Result<impl Responder> {
    let query = web::Query::<SearchQuery>::from_query(request.query_string())?;

    let results = search_index(&state.index, &query.query)
        .context("could not search index")
        .map_err(ise)?;

    //let results = state.index.send(SearchTantivyIndex(query.query.clone())).await
    //    .context("could not retrieve search results")
    //    .map_err(ise)?
    //    .context("could not search index")
    //    .map_err(ise)?;

    let mut response = HttpResponse::Ok();

    if query.ty == Some("json".to_string()) {
        Ok(response.json(results))
    } else {
        let meta = "<meta name=\"robots\" content=\"noindex\">";
        let resp = render_page(
            &state.config.web_root,
            "search",
            meta,
            &query.query,
            &results,
        )
        .map_err(ise)?;
        response.set_header(http::header::CONTENT_TYPE, "text/html");

        Ok(response.body(resp))
    }
}

fn integrity<P: AsRef<std::path::Path>>(file: P) -> anyhow::Result<String> {
    use ssri::{Algorithm, IntegrityOpts};

    // TODO probably don't need to do this on every request...
    let file = File::open(file).context("could not open file")?;
    let mut reader = BufReader::new(file);
    let mut content = Vec::new();
    reader
        .read_to_end(&mut content)
        .context("could not read file")?;

    let sha384 = IntegrityOpts::new()
        .algorithm(Algorithm::Sha384)
        .chain(content)
        .result()
        .to_string();

    Ok(sha384)
}

fn render_page(
    web_root: impl AsRef<path::Path>,
    body_classes: &str,
    extra_head: &str,
    query_str: &str,
    results: &[CmdletResult],
) -> anyhow::Result<Vec<u8>> {
    let web_root = web_root.as_ref();
    let style_path = web_root.join("static/style.css");
    let mut resp = Vec::new();
    let style_integrity =
        integrity(style_path).context("could not calculate integrity for stylesheet")?;
    //let script_integrity =
    //    integrity(js_path).context("could not calculate integrity for javascript")?;
    templates::index_html(
        &mut resp,
        &style_integrity,
        body_classes,
        extra_head,
        query_str,
        results,
    )
    .context("could not render index template")?;

    Ok(resp)
}

async fn index(
    state: web::Data<State>,
    _request: HttpRequest,
) -> actix_web::Result<impl Responder> {
    let resp = render_page(&state.config.web_root, "", "", "", &[]).map_err(ise)?;

    Ok(HttpResponse::Ok()
        .set_header(http::header::CONTENT_TYPE, "text/html")
        .body(resp))
}

async fn robots() -> actix_web::Result<impl Responder> {
    let robots = "User-Agent: *
Disallow: /search";
    Ok(HttpResponse::Ok()
        .set_header(http::header::CONTENT_TYPE, "text/plain")
        .body(robots))
}

struct State {
    index: tantivy::Index,
    //index: actix::prelude::Addr<TantivyIndexExecutor>,
    config: Config,
}

struct TantivyIndexExecutor(tantivy::Index);

impl actix::prelude::Actor for TantivyIndexExecutor {
    type Context = actix::prelude::SyncContext<Self>;
}

struct SearchTantivyIndex(String);

impl Message for SearchTantivyIndex {
    type Result = anyhow::Result<Vec<CmdletResult>>;
}

impl Handler<SearchTantivyIndex> for TantivyIndexExecutor {
    type Result = anyhow::Result<Vec<CmdletResult>>;

    fn handle(&mut self, query: SearchTantivyIndex, _: &mut Self::Context) -> Self::Result {
        search_index(&self.0, &query.0)
    }
}

fn error_handler(
    response: actix_web::dev::ServiceResponse,
) -> actix_web::Result<middleware::errhandlers::ErrorHandlerResponse<actix_web::dev::Body>> {
    let oh_no = r"             _______
            < oh no >
             -------
             /
      ____  /
 /\  / . .\  /\
 \ \/   _  \/ /
  \          /
   |        |
   |        |

Something went wrong...";
    Ok(middleware::errhandlers::ErrorHandlerResponse::Response(
        response.map_body(|head, _body| {
            head.headers_mut().append(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/plain"),
            );
            actix_web::dev::ResponseBody::Body(oh_no.into())
        }),
    ))
}

fn run_server(config: &Config) -> anyhow::Result<()> {
    let t_index = tantivy::Index::open_in_dir(&config.index_dir)
        .map_err(SearchError::Tantivy)
        .context("failed to open index directory")?;
    pascal_splitter::register(&t_index);

    //let todo = 1; // TODO should be num_cpu
    //let index_addr = SyncArbiter::start(todo, move || {
    //    TantivyIndexExecutor(t_index.clone())
    //});

    let server_config = config.clone();

    let mut server = HttpServer::new(move || {
        let static_dir = path::Path::new(&server_config.web_root).join("static");
        let assets_dir = path::Path::new(&server_config.web_root).join("assets");
        let typescript_dir = path::Path::new(&server_config.web_root).join("typescript");
        let sass_dir = path::Path::new(&server_config.web_root).join("sass");

        let mut default_headers = middleware::DefaultHeaders::new();
        for (key, value) in &server_config.headers {
            default_headers = default_headers.header(key, value);
        }

        let mut labels = HashMap::new();
        labels.insert(
            "host".to_string(),
            hostname::get()
                .map(|h| h.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "unknown".to_string()),
        );
        let prom = actix_web_prom::PrometheusMetrics::new("fcw", Some("/metrics"), Some(labels));

        App::new()
            .data(State {
                index: t_index.clone(),
                //index: index_addr.clone(),
                config: server_config.clone(),
            })
            .wrap(
                middleware::errhandlers::ErrorHandlers::new()
                    .handler(
                        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                        error_handler,
                    )
                    .handler(actix_web::http::StatusCode::NOT_FOUND, error_handler)
                    .handler(actix_web::http::StatusCode::FORBIDDEN, error_handler),
            )
            .wrap(prom)
            .wrap(middleware::Compress::default())
            .wrap(default_headers)
            .route("/", web::get().to(index))
            .route("/search", web::get().to(search))
            .route("/robots.txt", web::get().to(robots))
            .service(fs::Files::new("/static", static_dir))
            .service(fs::Files::new("/assets", assets_dir))
            .service(fs::Files::new("/typescript", typescript_dir))
            .service(fs::Files::new("/sass", sass_dir))
    });

    let mut fds = listenfd::ListenFd::from_env();
    let mut i = 0;
    let len = fds.len();
    while i < len {
        let listener = fds
            .take_tcp_listener(i)
            .context("failed to get tcp listener from provided fd")?
            .ok_or_else(|| anyhow::anyhow!("no tcp listener"))?;
        if listener
            .local_addr()
            .with_context(|| format!("could not get address for listener {}", i))?
            .port()
            == 443
        {
            let ssl_section = config
                .ssl
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no ssl section in configuration"))?;
            let mut acceptor =
                openssl::ssl::SslAcceptor::mozilla_modern_v5(openssl::ssl::SslMethod::tls())
                    .context("could not create SSL acceptor with modern defaults")?;
            acceptor
                .set_private_key_file(&ssl_section.private_key, openssl::ssl::SslFiletype::PEM)
                .context("could not set private key file")?;
            acceptor
                .set_certificate_chain_file(&ssl_section.certificate_chain)
                .context("could not set certificate chain")?;
            server = server
                .listen_openssl(listener, acceptor)
                .with_context(|| format!("could not listen with ssl on fd {}", i))?;
        } else {
            server = server
                .listen(listener)
                .with_context(|| format!("could not listen on fd {}", i))?;
        }
        i += 1;
    }

    if len == 0 {
        let listen_addr = config.listen_addr.as_deref().ok_or_else(|| {
            anyhow::anyhow!("listen-addr must be specified when sockets not passed")
        })?;
        server = server
            .bind(listen_addr)
            .with_context(|| format!("could not bind to address: {}", listen_addr))?;
    }

    server.run();

    Ok(())
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    index_dir: String,
    web_root: String,
    listen_addr: Option<String>,
    ssl: Option<SslConfig>,
    headers: std::collections::HashMap<String, String>,
}

impl Config {
    fn from_file(path: impl AsRef<path::Path>) -> anyhow::Result<Config> {
        let file = File::open(path).context("could not open config file")?;
        let mut buf_reader = BufReader::new(file);
        let mut text = String::new();
        buf_reader
            .read_to_string(&mut text)
            .context("could not read config file")?;

        let config = toml::from_str(&text).context("could not parse config file")?;

        Ok(config)
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SslConfig {
    private_key: String,
    certificate_chain: String,
}

/*
   TODO
    * Improve resonsive styling (search box/button)
*/

fn main() -> anyhow::Result<()> {
    let matches = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("FILE")
                .help("Configuration file for find-cmdlet-web"),
        )
        .get_matches();

    let config = matches
        .value_of("config")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            std::env::var("FCW_CONFIG").unwrap_or_else(|_| "Config.toml".to_string())
        });

    let config = Config::from_file(&config)
        .with_context(|| format!("could not load config file: {}", &config))?;

    pretty_env_logger::formatted_timed_builder()
        .filter_module("find_cmdlet_web", log::LevelFilter::Trace)
        .init();

    let system = actix_rt::System::new("find-cmdlet-web");
    run_server(&config).context("could not run server")?;

    system.run().context("could not run system")
}
