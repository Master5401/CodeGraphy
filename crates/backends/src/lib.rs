#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use atlas_engine::{
    EdgeKind, ExtractionMode, IndexPipeline, IndexPipelineOptions, NoopSink, Provenance,
    ReferenceKind, ResolutionStrategy, Store,
};
use serde::Serialize;
use serde_json::Value;

pub const ATLAS_VERSION: &str = "1.7.0";
pub const ATLAS_COMMIT: &str = "0f8379ff61a6f5ea1ae888d331f21b8b44f6a5b7";
pub const REPORT_SCHEMA: &str = "codegraphy-analysis/v1";

#[derive(Clone, Debug, Serialize)]
pub struct Evidence {
    pub provider: String,
    pub provider_version: String,
    pub provider_fact_id: String,
    pub path: String,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub resolution_state: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArtifactFact {
    pub id: String,
    pub path: String,
    pub language: String,
    pub content_hash: String,
    pub parse_status: String,
    pub evidence: Evidence,
    pub raw_provider_fact: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct SymbolFact {
    pub id: String,
    pub kind: String,
    pub language: String,
    pub name: String,
    pub qualified_name: String,
    pub signature: Option<String>,
    pub provider_symbol_id: String,
    pub evidence: Evidence,
    pub raw_provider_fact: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct RelationFact {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub target: String,
    pub source_name: String,
    pub target_name: String,
    pub provider_edge_id: String,
    pub confidence: f32,
    pub resolution_state: String,
    pub unresolved_reason: Option<String>,
    pub evidence: Option<Evidence>,
    pub raw_provider_fact: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnresolvedReferenceFact {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub receiver: Option<String>,
    pub arity: Option<u32>,
    pub source_symbol: Option<String>,
    pub evidence: Evidence,
    pub unresolved_reason: String,
    pub raw_provider_fact: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderDiagnostic {
    pub provider: String,
    pub code: String,
    pub message: String,
    pub resolution_state: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PipelineSummary {
    pub discovered_files: usize,
    pub indexed_files: usize,
    pub cached_files: usize,
    pub failed_files: usize,
    pub symbols: usize,
    pub edges: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisReport {
    pub schema: &'static str,
    pub provider: &'static str,
    pub provider_version: &'static str,
    pub provider_commit: &'static str,
    pub profile: &'static str,
    pub summary: PipelineSummary,
    pub artifacts: Vec<ArtifactFact>,
    pub symbols: Vec<SymbolFact>,
    pub relations: Vec<RelationFact>,
    pub unresolved_calls: Vec<UnresolvedReferenceFact>,
    pub diagnostics: Vec<ProviderDiagnostic>,
}

/// Run Atlas's repository pipeline for Java source files and preserve both
/// provider facts and normalized, evidence-bearing records.
pub fn analyze_java_workspace(root: &Path, atlas_database: &Path) -> Result<AnalysisReport> {
    let root = fs::canonicalize(root)
        .with_context(|| format!("cannot resolve workspace {}", root.display()))?;
    if !root.is_dir() {
        bail!("workspace is not a directory: {}", root.display());
    }
    if let Some(parent) = atlas_database.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create Atlas state directory {}", parent.display()))?;
    }
    let database_parent = atlas_database
        .parent()
        .ok_or_else(|| anyhow!("Atlas database path has no parent"))?;
    let canonical_database_parent = fs::canonicalize(database_parent).with_context(|| {
        format!(
            "cannot resolve Atlas state directory {}",
            database_parent.display()
        )
    })?;
    let canonical_database = canonical_database_parent.join(
        atlas_database
            .file_name()
            .ok_or_else(|| anyhow!("Atlas database path has no filename"))?,
    );
    let codewiki_state = fs::canonicalize(root.join(".codewiki"))
        .context("cannot resolve the workspace .codewiki state directory")?;
    if !codewiki_state.starts_with(&root) {
        bail!("workspace .codewiki state directory resolves outside the workspace");
    }
    if canonical_database.starts_with(&root) && !canonical_database.starts_with(&codewiki_state) {
        bail!("Atlas state inside a workspace is allowed only under .codewiki");
    }

    let store = Arc::new(Store::open_db(&canonical_database).with_context(|| {
        format!(
            "cannot open Atlas database {}",
            canonical_database.display()
        )
    })?);
    store
        .init_schema()
        .context("cannot initialize Atlas schema")?;

    let options = IndexPipelineOptions::new(ExtractionMode::Structural)
        .with_include_patterns(vec!["**/*.java".to_owned()]);
    let pipeline = IndexPipeline::new(Arc::clone(&store), root.clone(), options);
    let mut interrupted = || false;
    let stats = pipeline
        .run(&NoopSink, &mut interrupted)
        .context("Atlas Java structural analysis failed")?;

    let files = store.list_files().context("cannot read Atlas file facts")?;
    let file_paths: HashMap<String, String> = files
        .iter()
        .map(|file| {
            (
                file.file_id.to_string(),
                normalize_relative_path(&file.path),
            )
        })
        .collect();
    let mut artifacts = Vec::with_capacity(files.len());
    for file in &files {
        let path = normalize_relative_path(&file.path);
        artifacts.push(ArtifactFact {
            id: stable_id("artifact", &[&path]),
            evidence: Evidence {
                provider: "atlas-engine".to_owned(),
                provider_version: ATLAS_VERSION.to_owned(),
                provider_fact_id: file.file_id.to_string(),
                path: path.clone(),
                line_start: None,
                line_end: None,
                resolution_state: "RESOLVED".to_owned(),
            },
            path,
            language: file.language.as_str().to_owned(),
            content_hash: file.content_hash.clone(),
            parse_status: format!("{:?}", file.status),
            raw_provider_fact: serde_json::to_value(file)?,
        });
    }
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));

    let provider_symbols = store
        .get_all_symbols()
        .context("cannot read Atlas symbols")?;
    let mut symbol_ids = HashMap::with_capacity(provider_symbols.len());
    let mut symbol_names = HashMap::with_capacity(provider_symbols.len());
    let mut symbols = Vec::with_capacity(provider_symbols.len());
    for symbol in &provider_symbols {
        let path = file_paths
            .get(&symbol.file_id.to_string())
            .cloned()
            .ok_or_else(|| anyhow!("Atlas symbol {} has no indexed source file", symbol.id))?;
        let symbol_id = stable_id(
            "symbol",
            &[
                &path,
                symbol.language.as_str(),
                &symbol.qualified_name,
                symbol.signature.as_deref().unwrap_or_default(),
            ],
        );
        symbol_ids.insert(symbol.id.to_string(), symbol_id.clone());
        symbol_names.insert(symbol.id.to_string(), symbol.qualified_name.clone());
        symbols.push(SymbolFact {
            id: symbol_id,
            kind: symbol.kind.as_str().to_owned(),
            language: symbol.language.as_str().to_owned(),
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            signature: symbol.signature.clone(),
            provider_symbol_id: symbol.id.to_string(),
            evidence: Evidence {
                provider: "atlas-engine".to_owned(),
                provider_version: ATLAS_VERSION.to_owned(),
                provider_fact_id: symbol.id.to_string(),
                path,
                line_start: Some(symbol.range.start_line + 1),
                line_end: Some(symbol.range.end_line + 1),
                resolution_state: "RESOLVED".to_owned(),
            },
            raw_provider_fact: serde_json::to_value(symbol)?,
        });
    }
    symbols.sort_by(|left, right| {
        left.evidence
            .path
            .cmp(&right.evidence.path)
            .then_with(|| left.evidence.line_start.cmp(&right.evidence.line_start))
            .then_with(|| left.qualified_name.cmp(&right.qualified_name))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut provider_references = Vec::new();
    let mut reference_paths = HashMap::new();
    for file in &files {
        let path = normalize_relative_path(&file.path);
        for reference in store
            .find_references_by_file(&file.file_id)
            .with_context(|| format!("cannot read references for {path}"))?
        {
            reference_paths.insert(reference.id.to_string(), path.clone());
            provider_references.push(reference);
        }
    }
    provider_references.sort_by_key(|reference| reference.id.to_string());
    let reference_by_id: HashMap<String, atlas_engine::ReferenceUse> = provider_references
        .iter()
        .cloned()
        .map(|reference| (reference.id.to_string(), reference))
        .collect();

    let provider_edges = store.get_all_edges().context("cannot read Atlas edges")?;
    let mut relations = Vec::with_capacity(provider_edges.len());
    for edge in &provider_edges {
        let source = symbol_ids.get(&edge.source.to_string());
        let target = symbol_ids.get(&edge.target.to_string());
        let Some((source, target)) = source.zip(target) else {
            continue;
        };
        let evidence_location = edge
            .location
            .map(|range| (edge.ref_id.as_ref().map(ToString::to_string), range))
            .or_else(|| {
                edge.ref_id.as_ref().and_then(|reference_id| {
                    reference_by_id
                        .get(&reference_id.to_string())
                        .map(|reference| (Some(reference.id.to_string()), reference.range))
                })
            });
        let evidence = evidence_location.and_then(|(reference_id, range)| {
            let provider_fact_id = reference_id.unwrap_or_else(|| edge.id.to_string());
            let path = reference_paths
                .get(&provider_fact_id)
                .or_else(|| {
                    provider_symbols
                        .iter()
                        .find(|symbol| symbol.id == edge.source)
                        .and_then(|symbol| file_paths.get(&symbol.file_id.to_string()))
                })?
                .clone();
            Some(Evidence {
                provider: "atlas-engine".to_owned(),
                provider_version: ATLAS_VERSION.to_owned(),
                provider_fact_id,
                path,
                line_start: Some(range.start_line + 1),
                line_end: Some(range.end_line + 1),
                resolution_state: relation_resolution(edge).to_owned(),
            })
        });
        let resolution_state = if evidence.is_some() {
            relation_resolution(edge)
        } else {
            "UNRESOLVED"
        };
        let source_name = symbol_names
            .get(&edge.source.to_string())
            .ok_or_else(|| anyhow!("Atlas relation {} has no source symbol", edge.id))?
            .clone();
        let target_name = symbol_names
            .get(&edge.target.to_string())
            .ok_or_else(|| anyhow!("Atlas relation {} has no target symbol", edge.id))?
            .clone();
        let unresolved_reason = (resolution_state == "UNRESOLVED").then(|| {
            if evidence.is_none() {
                "Atlas edge has no source range or source-file evidence.".to_owned()
            } else {
                "Atlas did not report an exact or import-based resolution strategy.".to_owned()
            }
        });
        relations.push(RelationFact {
            id: stable_id("relation", &[&edge.id.to_string()]),
            kind: edge.kind.as_str().to_owned(),
            source: source.clone(),
            target: target.clone(),
            source_name,
            target_name,
            provider_edge_id: edge.id.to_string(),
            confidence: edge.confidence.as_f32(),
            resolution_state: resolution_state.to_owned(),
            unresolved_reason,
            evidence,
            raw_provider_fact: serde_json::to_value(edge)?,
        });
    }
    relations.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut unresolved_calls = Vec::new();
    for reference in provider_references
        .iter()
        .filter(|reference| reference.kind == ReferenceKind::Call && reference.resolved.is_none())
    {
        let path = reference_paths
            .get(&reference.id.to_string())
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "Atlas reference {} has no indexed source file",
                    reference.id
                )
            })?;
        unresolved_calls.push(UnresolvedReferenceFact {
            id: stable_id("unresolved-call", &[&reference.id.to_string()]),
            kind: reference.kind.as_str().to_owned(),
            name: reference.name.clone(),
            receiver: reference.receiver.clone(),
            arity: reference.arity,
            source_symbol: reference
                .source_symbol
                .as_ref()
                .and_then(|id| symbol_ids.get(&id.to_string()).cloned()),
            evidence: Evidence {
                provider: "atlas-engine".to_owned(),
                provider_version: ATLAS_VERSION.to_owned(),
                provider_fact_id: reference.id.to_string(),
                path,
                line_start: Some(reference.range.start_line + 1),
                line_end: Some(reference.range.end_line + 1),
                resolution_state: "UNRESOLVED".to_owned(),
            },
            unresolved_reason: "Atlas reported no resolved target for this callsite.".to_owned(),
            raw_provider_fact: serde_json::to_value(reference)?,
        });
    }
    unresolved_calls.sort_by(|left, right| {
        left.evidence
            .path
            .cmp(&right.evidence.path)
            .then_with(|| left.evidence.line_start.cmp(&right.evidence.line_start))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });

    let diagnostics = if stats.failed > 0 {
        vec![ProviderDiagnostic {
            provider: "atlas-engine".to_owned(),
            code: "atlas_files_failed".to_owned(),
            message: format!(
                "Atlas reports {} failed Java files; results are partial.",
                stats.failed
            ),
            resolution_state: "UNRESOLVED".to_owned(),
        }]
    } else {
        Vec::new()
    };

    Ok(AnalysisReport {
        schema: REPORT_SCHEMA,
        provider: "atlas-engine",
        provider_version: ATLAS_VERSION,
        provider_commit: ATLAS_COMMIT,
        profile: "java-structural",
        summary: PipelineSummary {
            discovered_files: stats.discovered,
            indexed_files: stats.indexed,
            cached_files: stats
                .discovered
                .saturating_sub(stats.indexed.saturating_add(stats.failed)),
            failed_files: stats.failed,
            symbols: symbols.len(),
            edges: relations.len(),
        },
        artifacts,
        symbols,
        relations,
        unresolved_calls,
        diagnostics,
    })
}

/// Write deterministic, source-shaped reports without modifying source files.
pub fn write_reports(report: &AnalysisReport, output: &Path) -> Result<()> {
    fs::create_dir_all(output)
        .with_context(|| format!("cannot create report directory {}", output.display()))?;
    write_file_atomic(
        &output.join("facts.json"),
        &format!("{}\n", serde_json::to_string_pretty(report)?),
    )?;
    write_file_atomic(&output.join("call-graph.dot"), &render_call_graph(report))?;
    write_file_atomic(&output.join("report.md"), &render_summary(report))?;

    for artifact in &report.artifacts {
        let relative = Path::new(&artifact.path);
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            bail!("Atlas returned an unsafe relative path: {}", artifact.path);
        }
        let mut page_path = output.join("files").join(relative);
        let filename = page_path
            .file_name()
            .ok_or_else(|| anyhow!("file path has no filename: {}", artifact.path))?
            .to_string_lossy()
            .to_string();
        page_path.set_file_name(format!("{filename}.md"));
        let page = render_file_page(report, artifact);
        write_file_atomic(&page_path, &page)?;
    }
    Ok(())
}

fn relation_resolution(edge: &atlas_engine::RawEdge) -> &'static str {
    match edge.resolved_by {
        Some(ResolutionStrategy::ExactMatch | ResolutionStrategy::ImportResolved) => "RESOLVED",
        Some(ResolutionStrategy::Builtin) => "EXTERNAL",
        Some(
            ResolutionStrategy::NameOnly
            | ResolutionStrategy::FuzzyMatch
            | ResolutionStrategy::Heuristic
            | ResolutionStrategy::DataflowPointer,
        ) => "HEURISTIC",
        None if edge.kind == EdgeKind::Contains && edge.ref_id.is_none() => "RESOLVED",
        None => match edge.provenance {
            Provenance::Heuristic | Provenance::CallbackPattern => "HEURISTIC",
            Provenance::TreeSitter | Provenance::Scip | Provenance::UserAnnotation => "RESOLVED",
            Provenance::FocusClosure(_) => "UNRESOLVED",
        },
    }
}

fn render_call_graph(report: &AnalysisReport) -> String {
    let symbols: HashMap<_, _> = report
        .symbols
        .iter()
        .map(|symbol| (symbol.id.as_str(), symbol))
        .collect();
    let mut output = String::from("digraph codegraphy_calls {\n  rankdir=LR;\n");
    for symbol in &report.symbols {
        let label = format!("{}\\n{}", symbol.qualified_name, symbol.evidence.path);
        output.push_str(&format!(
            "  \"{}\" [label=\"{}\"];\n",
            dot_escape(&symbol.id),
            dot_escape(&label)
        ));
    }
    for relation in report.relations.iter().filter(|relation| {
        matches!(
            relation.kind.as_str(),
            "calls" | "instantiates" | "implements"
        ) && matches!(relation.resolution_state.as_str(), "RESOLVED" | "HEURISTIC")
            && relation.evidence.is_some()
    }) {
        let source = symbols
            .get(relation.source.as_str())
            .map(|symbol| symbol.qualified_name.as_str())
            .unwrap_or("unknown source");
        let target = symbols
            .get(relation.target.as_str())
            .map(|symbol| symbol.qualified_name.as_str())
            .unwrap_or("unknown target");
        let evidence = relation.evidence.as_ref().expect("filtered evidence");
        let line = evidence.line_start.unwrap_or_default();
        let style = if relation.resolution_state == "HEURISTIC" {
            ", style=dashed"
        } else {
            ""
        };
        output.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{} [{}]\"{}, tooltip=\"{}:{} → {}\"];\n",
            dot_escape(&relation.source),
            dot_escape(&relation.target),
            dot_escape(&relation.kind),
            dot_escape(&relation.resolution_state),
            style,
            dot_escape(&evidence.path),
            line,
            dot_escape(&format!("{source} → {target}")),
        ));
    }
    output.push_str("}\n");
    output
}

fn render_summary(report: &AnalysisReport) -> String {
    let resolved_calls = report
        .relations
        .iter()
        .filter(|relation| relation.kind == "calls" && relation.resolution_state == "RESOLVED")
        .count();
    let heuristic_calls = report
        .relations
        .iter()
        .filter(|relation| relation.kind == "calls" && relation.resolution_state == "HEURISTIC")
        .count();
    let mut output = format!(
        "# CodeGraphy scan\n\nProvider: Atlas {} (`{}`), Java structural profile.\n\nDiscovered {} Java files; re-extracted {}; reused cached analysis for {}; {} failed. Extracted {} symbols and {} relationships, including {} resolved internal calls, {} heuristic call candidates, and {} unresolved callsites.\n\n",
        report.provider_version,
        report.provider_commit,
        report.summary.discovered_files,
        report.summary.indexed_files,
        report.summary.cached_files,
        report.summary.failed_files,
        report.summary.symbols,
        report.summary.edges,
        resolved_calls,
        heuristic_calls,
        report.unresolved_calls.len(),
    );
    output.push_str("## Output files\n\n- `facts.json`: evidence-bearing normalized facts plus raw Atlas records.\n- `call-graph.dot`: solid resolved edges and dashed heuristic candidates.\n- `files/`: one source-shaped facts page per analyzed Java file.\n\n");
    if !report.diagnostics.is_empty() {
        output.push_str("## Diagnostics\n\n");
        for diagnostic in &report.diagnostics {
            output.push_str(&format!("- {}: {}\n", diagnostic.code, diagnostic.message));
        }
        output.push('\n');
    }
    output.push_str("The call graph draws resolved edges as solid lines and heuristic candidates as dashed lines. Unresolved references remain in `facts.json` with evidence and a reason; a missing graph edge does not mean the source contains no call.\n");
    output
}

fn render_file_page(report: &AnalysisReport, artifact: &ArtifactFact) -> String {
    let mut output = format!(
        "# `{}`\n\nAtlas Java structural facts. File hash: `{}`. Parse status: `{}`.\n\n",
        artifact.path, artifact.content_hash, artifact.parse_status
    );
    output.push_str("## Definitions\n\n");
    let mut symbols = report
        .symbols
        .iter()
        .filter(|symbol| symbol.evidence.path == artifact.path)
        .collect::<Vec<_>>();
    symbols.sort_by(|left, right| {
        left.evidence
            .line_start
            .cmp(&right.evidence.line_start)
            .then_with(|| left.qualified_name.cmp(&right.qualified_name))
    });
    if symbols.is_empty() {
        output.push_str("No symbol definitions were returned by Atlas for this file.\n\n");
    }
    for symbol in &symbols {
        output.push_str(&format!(
            "- `{}`{} — {} at lines {}–{}. Evidence: Atlas symbol `{}`.\n",
            symbol.qualified_name,
            symbol
                .signature
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_default(),
            symbol.kind,
            symbol.evidence.line_start.unwrap_or_default(),
            symbol.evidence.line_end.unwrap_or_default(),
            symbol.provider_symbol_id,
        ));
    }
    output.push_str("\n## Outgoing relationships\n\nResolved edges and heuristic candidates are labelled with their provider resolution state.\n\n");
    let symbol_names: HashMap<_, _> = report
        .symbols
        .iter()
        .map(|symbol| (symbol.id.as_str(), symbol.qualified_name.as_str()))
        .collect();
    let mut outgoing = report
        .relations
        .iter()
        .filter(|relation| {
            matches!(relation.resolution_state.as_str(), "RESOLVED" | "HEURISTIC")
                && relation
                    .evidence
                    .as_ref()
                    .is_some_and(|e| e.path == artifact.path)
                && symbols.iter().any(|symbol| symbol.id == relation.source)
        })
        .collect::<Vec<_>>();
    outgoing.sort_by(|left, right| {
        left.evidence
            .as_ref()
            .and_then(|e| e.line_start)
            .cmp(&right.evidence.as_ref().and_then(|e| e.line_start))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.target.cmp(&right.target))
    });
    if outgoing.is_empty() {
        output
            .push_str("No resolved outgoing relationships with source evidence were returned.\n\n");
    }
    for relation in outgoing {
        let target = symbol_names
            .get(relation.target.as_str())
            .copied()
            .unwrap_or("unavailable target");
        let evidence = relation.evidence.as_ref().expect("filtered evidence");
        output.push_str(&format!(
            "- `{}` to `{target}` at line {}. Evidence: Atlas edge `{}`; state `{}`.\n",
            relation.kind,
            evidence.line_start.unwrap_or_default(),
            relation.provider_edge_id,
            relation.resolution_state,
        ));
    }
    let unresolved = report
        .unresolved_calls
        .iter()
        .filter(|reference| reference.evidence.path == artifact.path)
        .collect::<Vec<_>>();
    output.push_str("\n## Unresolved calls\n\n");
    if unresolved.is_empty() {
        output.push_str("No unresolved callsites were reported for this file.\n");
    }
    for reference in unresolved {
        output.push_str(&format!(
            "- `{}` at line {} remains unresolved: {} Evidence: Atlas reference `{}`.\n",
            reference.name,
            reference.evidence.line_start.unwrap_or_default(),
            reference.unresolved_reason,
            reference.evidence.provider_fact_id,
        ));
    }
    output
}

fn write_file_atomic(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("report output has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("cannot create report directory {}", parent.display()))?;
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("out")
    ));
    fs::write(&temporary, contents)
        .with_context(|| format!("cannot write report file {}", temporary.display()))?;
    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("cannot replace report {}", path.display()))?;
    }
    fs::rename(&temporary, path)
        .with_context(|| format!("cannot finalize report {}", path.display()))
}

fn stable_id(kind: &str, parts: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(kind.as_bytes());
    for part in parts {
        hasher.update(&[0]);
        hasher.update(part.as_bytes());
    }
    format!("{kind}:{}", hasher.finalize().to_hex())
}

fn normalize_relative_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn dot_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn synthetic_java_workspace() -> (TempDir, PathBuf) {
        let temp = tempfile::tempdir().expect("temp workspace");
        let source_dir = temp.path().join("src/main/java/proof");
        fs::create_dir_all(&source_dir).expect("create source directory");
        fs::write(
            source_dir.join("PriceFlow.java"),
            "package proof;\npublic final class PriceFlow {\n    public int total(int quantity, int unitPrice) {\n        int subtotal = quantity * unitPrice;\n        return addFee(subtotal);\n    }\n    private int addFee(int amount) {\n        int fee = 2;\n        return amount + fee;\n    }\n    private int missingTarget(int value) {\n        return absent(value);\n    }\n}\n",
        )
        .expect("write synthetic Java fixture");
        let database = temp.path().join(".codewiki/atlas/atlas.db");
        (temp, database)
    }

    #[test]
    fn atlas_analysis_returns_evidence_backed_call_edges_and_unresolved_calls() {
        let (workspace, database) = synthetic_java_workspace();
        let report = analyze_java_workspace(workspace.path(), &database).expect("Atlas analysis");

        assert_eq!(report.provider, "atlas-engine");
        assert_eq!(report.profile, "java-structural");
        assert!(
            report
                .symbols
                .iter()
                .any(|symbol| symbol.qualified_name.ends_with("PriceFlow.total"))
        );
        let resolved_call = report
            .relations
            .iter()
            .find(|relation| {
                relation.kind == "calls"
                    && matches!(relation.resolution_state.as_str(), "RESOLVED" | "HEURISTIC")
                    && report.symbols.iter().any(|symbol| {
                        symbol.id == relation.source && symbol.qualified_name.ends_with(".total")
                    })
            })
            .unwrap_or_else(|| {
                panic!(
                    "call candidate not produced; summary={:?}; symbols={:?}; relations={:?}; unresolved={:?}",
                    report.summary,
                    report.symbols.iter().map(|symbol| &symbol.qualified_name).collect::<Vec<_>>(),
                    report.relations.iter().map(|relation| (&relation.kind, &relation.resolution_state, &relation.unresolved_reason)).collect::<Vec<_>>(),
                    report.unresolved_calls.iter().map(|reference| (&reference.name, &reference.evidence.path, &reference.evidence.line_start)).collect::<Vec<_>>(),
                )
            });
        assert!(resolved_call.evidence.is_some());
        assert!(
            report
                .unresolved_calls
                .iter()
                .any(|reference| reference.name == "absent")
        );
    }

    #[test]
    fn report_files_are_deterministic_and_mirror_source_paths() {
        let (workspace, database) = synthetic_java_workspace();
        let report = analyze_java_workspace(workspace.path(), &database).expect("Atlas analysis");
        let output = workspace.path().join(".codewiki/generated");
        write_reports(&report, &output).expect("write reports");
        let first_json = fs::read(output.join("facts.json")).expect("read facts");
        let first_graph = fs::read(output.join("call-graph.dot")).expect("read graph");
        write_reports(&report, &output).expect("repeat reports");
        assert_eq!(first_json, fs::read(output.join("facts.json")).unwrap());
        assert_eq!(
            first_graph,
            fs::read(output.join("call-graph.dot")).unwrap()
        );
        let dot = String::from_utf8(first_graph).expect("DOT is UTF-8");
        assert!(dot.contains("style=dashed"));
        assert!(dot.contains("HEURISTIC"));
        assert!(
            output
                .join("files/src/main/java/proof/PriceFlow.java.md")
                .is_file()
        );
    }
}
