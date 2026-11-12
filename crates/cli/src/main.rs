use std::fmt::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use codewiki_backends::{analyze_java_workspace, write_reports};
use codewiki_core::WorkspaceStore;
use codewiki_server::{ServerState, serve};
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(
    name = "codegraphy",
    version,
    about = "Local codebase catalog and comprehension UI"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    /// Workspace folder to open in the browser app; no arguments scan the current directory.
    workspace: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize CodeGraphy state, catalog a workspace, analyze Java, and write reports.
    #[command(alias = "scan")]
    Init {
        /// Workspace folder to analyze.
        workspace: PathBuf,
        /// Report output folder. Defaults to <workspace>/.codewiki/analysis.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Open the local browser app, optionally with a workspace already selected.
    Ui {
        /// Workspace folder to open in the app.
        workspace: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let workspace = match args.command {
        Some(Command::Init { workspace, output }) => return scan_workspace(workspace, output),
        Some(Command::Ui { workspace }) => workspace.or(args.workspace),
        None => match args.workspace {
            Some(workspace) => Some(workspace),
            None => return scan_workspace(std::env::current_dir()?, None),
        },
    };
    let session_token = create_session_token()?;
    let state = ServerState::new(session_token.clone());

    if let Some(workspace) = workspace {
        let store = WorkspaceStore::open(&workspace)
            .with_context(|| format!("could not open workspace {}", workspace.display()))?;
        println!("Opening CodeGraphy for {}", store.root().display());
        state.start_index(store)?;
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("could not bind CodeGraphy to loopback")?;
    let address = listener.local_addr()?;
    let SocketAddr::V4(address) = address else {
        anyhow::bail!("CodeGraphy requires an IPv4 loopback address");
    };
    state.set_port(address.port());

    let url = format!("http://127.0.0.1:{}/#{session_token}", address.port());
    println!(
        "CodeGraphy is available at http://127.0.0.1:{}",
        address.port()
    );
    if webbrowser::open(&url).is_err() {
        println!("Open this local URL in your browser: {url}");
    }

    let static_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/dist")
        .canonicalize()
        .context("web build not found; run `npm run build` in web/")?;
    serve(listener, state, static_root).await
}

fn scan_workspace(workspace: PathBuf, output: Option<PathBuf>) -> Result<()> {
    println!("Cataloging workspace files…");
    let store = WorkspaceStore::open(&workspace)
        .with_context(|| format!("could not open workspace {}", workspace.display()))?;
    let catalog = store
        .index_workspace(|_| {})
        .context("workspace catalog scan failed")?;
    let state_dir = store
        .database_path()
        .parent()
        .context("workspace database has no state directory")?;
    let atlas_database = state_dir.join("atlas/atlas.db");
    println!("Analyzing Java source with Atlas…");
    let report = analyze_java_workspace(store.root(), &atlas_database)?;
    let report_dir = output.unwrap_or_else(|| state_dir.join("analysis"));
    write_reports(&report, &report_dir)?;
    store.save_analysis_report(&serde_json::to_value(&report)?)?;

    println!(
        "Cataloged {} files ({} text-indexed; {} diagnostics).",
        catalog.files_indexed, catalog.text_indexed, catalog.diagnostics
    );
    println!(
        "Atlas {} found {} Java files ({} re-extracted, {} cached, {} failed): {} symbols, {} relationships, {} unresolved calls.",
        report.provider_version,
        report.summary.discovered_files,
        report.summary.indexed_files,
        report.summary.cached_files,
        report.summary.failed_files,
        report.summary.symbols,
        report.summary.edges,
        report.unresolved_calls.len()
    );
    println!("Reports: {}", report_dir.display());
    println!(
        "Call graph: {}",
        report_dir.join("call-graph.dot").display()
    );
    Ok(())
}

fn create_session_token() -> Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).context("could not create local session token")?;
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut token, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(token)
}
