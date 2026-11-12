use std::fs;
use std::path::PathBuf;

use atlas_engine::{Engine, ExtractionMode, Language, SourcePath, decode_source};

fn main() -> anyhow::Result<()> {
    let fixture = PathBuf::from(std::env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("usage: atlas-proof <synthetic-java-source> <output-json>")
    })?);
    let output = PathBuf::from(std::env::args().nth(2).ok_or_else(|| {
        anyhow::anyhow!("usage: atlas-proof <synthetic-java-source> <output-json>")
    })?);
    let bytes = fs::read(&fixture)?;
    let source = decode_source(&bytes);
    let engine = Engine::open_in_memory()?;
    let facts = engine.extract_file_with_mode(
        &SourcePath::from_relative("src/main/java/proof/PriceFlow.java"),
        &source,
        Language::Java,
        ExtractionMode::Full,
    )?;
    let raw = serde_json::to_vec_pretty(&facts)?;
    fs::write(output, raw)?;
    eprintln!(
        "Atlas facade extracted {} symbols, {} references, {} callsites, {} CFG nodes, {} CFG edges, {} data nodes, {} dataflow edges, {} diagnostics",
        facts.symbols.len(),
        facts.references.len(),
        facts.callsites.len(),
        facts.cfg_nodes.len(),
        facts.cfg_edges.len(),
        facts.data_nodes.len(),
        facts.dataflow_edges.len(),
        facts.diagnostics.len(),
    );
    Ok(())
}
