import { FormEvent, ReactNode, useCallback, useEffect, useState } from "react";
import "./App.css";

type RuntimeProgress = {
  state: string;
  files_seen: number;
  files_indexed: number;
  text_indexed: number;
  diagnostics: number;
  error: string | null;
};

type WorkspaceStatus = {
  open: boolean;
  root: string | null;
  indexed_files: number;
  progress: RuntimeProgress;
};

type FileRecord = {
  id: string;
  workspace_id: string;
  repository_id: string;
  path: string;
  display_name: string;
  extension: string | null;
  language: string | null;
  detected_format: string;
  byte_size: number;
  content_hash: string | null;
  is_binary: boolean;
  capability_level: number;
  parse_status: string;
  classification_method: string;
  diagnostic: string | null;
};

type SearchHit = {
  file: FileRecord;
  snippet: string | null;
};

type WorkspaceEntry = {
  name: string;
  path: string;
  is_directory: boolean;
  file: FileRecord | null;
};

type DirectoryListing = {
  path: string;
  entries: WorkspaceEntry[];
  has_more: boolean;
};

type SourcePreview = {
  path: string;
  content: string;
  truncated: boolean;
};

type AnalysisCall = {
  kind: string;
  source_name: string;
  target_name: string;
  resolution_state: string;
  evidence: { path: string; line_start: number | null } | null;
};

type AnalysisOverview = {
  run: {
    provider_version: string;
    profile: string;
    summary: {
      discovered_files: number;
      indexed_files: number;
      failed_files: number;
      symbols: number;
      edges: number;
    };
  } | null;
  calls: AnalysisCall[];
  unresolved_calls: { name: string; evidence: { path: string; line_start: number | null }; unresolved_reason: string }[];
};

declare global {
  interface Window {
    __CODEWIKI_TOKEN__?: string;
  }
}

const bootToken = window.location.hash.slice(1);
if (bootToken) {
  window.__CODEWIKI_TOKEN__ = bootToken;
  window.history.replaceState(null, "", window.location.pathname);
}

async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = window.__CODEWIKI_TOKEN__;
  if (!token) {
    throw new Error("Launch CodeGraphy from the checkout with `.\codegraphy.ps1 <workspace>` or `./codegraphy <workspace>`.");
  }
  const headers = new Headers(init.headers);
  headers.set("X-Codewiki-Token", token);
  if (init.body) {
    headers.set("Content-Type", "application/json");
  }
  const response = await fetch(path, { ...init, headers });
  if (!response.ok) {
    const error = (await response.json().catch(() => null)) as { error?: string } | null;
    throw new Error(error?.error ?? `Request failed (${response.status})`);
  }
  if (response.status === 204 || response.headers.get("content-length") === "0") {
    return undefined as T;
  }
  return (await response.json()) as T;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function fileKind(file: FileRecord): string {
  if (file.language) return file.language;
  return file.detected_format.replaceAll("_", " ");
}

export default function App() {
  const [status, setStatus] = useState<WorkspaceStatus | null>(null);
  const [files, setFiles] = useState<SearchHit[]>([]);
  const [tree, setTree] = useState<Record<string, DirectoryListing>>({});
  const [expandedDirectories, setExpandedDirectories] = useState<Set<string>>(new Set());
  const [loadingDirectories, setLoadingDirectories] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState("");
  const [workspacePath, setWorkspacePath] = useState("");
  const [selected, setSelected] = useState<FileRecord | null>(null);
  const [preview, setPreview] = useState<SourcePreview | null>(null);
  const [analysis, setAnalysis] = useState<AnalysisOverview | null>(null);
  const [loading, setLoading] = useState(false);
  const [opening, setOpening] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadStatus = useCallback(async () => {
    try {
      const next = await api<WorkspaceStatus>("/api/status");
      setStatus(next);
      if (!next.open) {
        setFiles([]);
        setTree({});
        setExpandedDirectories(new Set());
        setLoadingDirectories(new Set());
        setSelected(null);
        setPreview(null);
        setAnalysis(null);
      }
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not reach the local CodeGraphy service.");
    }
  }, []);

  const loadAnalysis = useCallback(async () => {
    try {
      setAnalysis(await api<AnalysisOverview>("/api/analysis"));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not load Java analysis.");
    }
  }, []);

  const loadDirectory = useCallback(async (path: string, offset = 0) => {
    setLoadingDirectories((current) => new Set(current).add(path));
    try {
      const params = new URLSearchParams({ path, limit: "200", offset: String(offset) });
      const listing = await api<DirectoryListing>(`/api/tree?${params.toString()}`);
      setTree((current) => {
        const previous = current[path];
        const entries = offset > 0 && previous
          ? [...previous.entries, ...listing.entries]
          : listing.entries;
        return { ...current, [path]: { ...listing, entries } };
      });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not load the repository tree.");
    } finally {
      setLoadingDirectories((current) => {
        const next = new Set(current);
        next.delete(path);
        return next;
      });
    }
  }, []);

  useEffect(() => {
    void loadStatus();
    const timer = window.setInterval(() => void loadStatus(), 1500);
    return () => window.clearInterval(timer);
  }, [loadStatus]);

  useEffect(() => {
    if (!status?.open) return;
    let cancelled = false;
    setLoading(true);
    const timer = window.setTimeout(() => {
      const params = new URLSearchParams({ limit: "100" });
      if (query.trim()) params.set("q", query.trim());
      api<SearchHit[]>(`/api/files?${params.toString()}`)
        .then((results) => {
          if (!cancelled) setFiles(results);
        })
        .catch((reason: unknown) => {
          if (!cancelled) setError(reason instanceof Error ? reason.message : "Search failed.");
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, 160);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query, status?.open, status?.progress.files_indexed]);

  useEffect(() => {
    if (!status?.open) return;
    const directories = new Set(["", ...expandedDirectories]);
    for (const path of directories) void loadDirectory(path);
  }, [loadDirectory, status?.open, status?.progress.files_indexed, expandedDirectories]);

  useEffect(() => {
    if (status?.open && status.progress.state === "ready") void loadAnalysis();
  }, [loadAnalysis, status?.open, status?.progress.state]);

  async function openWorkspace(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setOpening(true);
    setError(null);
    try {
      await api<void>("/api/workspace", {
        method: "POST",
        body: JSON.stringify({ path: workspacePath.trim() }),
      });
      await loadStatus();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not open that folder.");
    } finally {
      setOpening(false);
    }
  }

  async function selectFile(file: FileRecord) {
    setSelected(file);
    setPreview(null);
    setError(null);
    if (file.is_binary) return;
    try {
      setPreview(await api<SourcePreview>(`/api/source/${encodeURIComponent(file.id)}`));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Source preview is unavailable.");
    }
  }

  function toggleDirectory(path: string) {
    setExpandedDirectories((current) => {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
    if (!expandedDirectories.has(path) && !tree[path]) void loadDirectory(path);
  }

  function renderTree(path: string, depth: number): ReactNode {
    const listing = tree[path];
    if (!listing) {
      return <div className="tree-loading">{loadingDirectories.has(path) ? "Loading…" : "No entries yet"}</div>;
    }
    return <>
      {listing.entries.map((entry) => (
        <div key={entry.path}>
          <button
            className={`tree-entry ${entry.is_directory ? "tree-directory" : "tree-file"} ${selected?.path === entry.path ? "selected" : ""}`}
            style={{ paddingLeft: `${9 + depth * 12}px` }}
            onClick={() => entry.is_directory ? toggleDirectory(entry.path) : entry.file && void selectFile(entry.file)}
            aria-expanded={entry.is_directory ? expandedDirectories.has(entry.path) : undefined}
            title={entry.path}
          >
            <span className="tree-chevron" aria-hidden="true">{entry.is_directory ? (expandedDirectories.has(entry.path) ? "▾" : "▸") : "·"}</span>
            <span className="tree-entry-name">{entry.name}</span>
            {entry.file && <span className={`level ${entry.file.capability_level ? "level-one" : "level-zero"}`}>L{entry.file.capability_level}</span>}
          </button>
          {entry.is_directory && expandedDirectories.has(entry.path) && renderTree(entry.path, depth + 1)}
        </div>
      ))}
      {listing.has_more && <button className="tree-load-more" disabled={loadingDirectories.has(path)} onClick={() => void loadDirectory(path, listing.entries.length)}>{loadingDirectories.has(path) ? "Loading…" : "Load more"}</button>}
      {!listing.entries.length && !loadingDirectories.has(path) && <div className="tree-loading">No files here</div>}
    </>;
  }

  if (!status?.open) {
    return (
      <main className="app-shell picker-shell">
        <header className="topbar">
          <div className="brand"><span className="brand-mark">C</span><span>CodeGraphy</span></div>
          <span className="local-label"><span className="status-dot" /> Local workspace</span>
        </header>
        <section className="picker-card">
          <p className="eyebrow">A clearer view of your code</p>
          <h1>Open a workspace</h1>
          <p className="muted intro-copy">Catalog files, search readable text, inspect source, and run Atlas Java analysis locally.</p>
          <form className="workspace-form" onSubmit={openWorkspace}>
            <label htmlFor="workspace-path">Workspace folder path</label>
            <div className="path-row">
              <input
                id="workspace-path"
                autoFocus
                value={workspacePath}
                onChange={(event) => setWorkspacePath(event.target.value)}
                placeholder="C:\\Projects\\my-app"
                required
                spellCheck={false}
              />
              <button className="primary-button" type="submit" disabled={opening || !workspacePath.trim()}>
                {opening ? "Opening…" : "Open workspace"}
              </button>
            </div>
          </form>
          {error && <p className="error-banner" role="alert">{error}</p>}
          <div className="privacy-note"><span aria-hidden="true">◉</span><span>Files stay on this machine. Indexing does not run project code or build scripts.</span></div>
        </section>
        <footer className="app-footer">CodeGraphy · Local and private</footer>
      </main>
    );
  }

  const progress = status.progress;
  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand"><span className="brand-mark">C</span><span>CodeGraphy</span></div>
        <div className="workspace-chip" title={status.root ?? ""}><span className="folder-glyph">▰</span><span className="workspace-name">{status.root}</span></div>
        <div className="local-label"><span className={`status-dot ${progress.state === "failed" ? "status-dot-error" : ""}`} />{progress.state === "indexing" ? "Indexing" : progress.state === "failed" ? "Needs attention" : "Local workspace"}</div>
      </header>

      <div className="main-layout">
        <aside className="sidebar">
          <div className="sidebar-heading"><span>WORKSPACE</span><button className="icon-button" onClick={() => { setStatus(null); setError(null); }} title="Choose another workspace" aria-label="Choose another workspace">＋</button></div>
          <div className="repo-root"><span className="repo-icon">⌂</span><span className="repo-label">{status.root?.split(/[\\/]/).filter(Boolean).at(-1) ?? "Workspace"}</span></div>
          <div className="side-section-label">INDEX</div>
          <div className="side-stat"><span>Files cataloged</span><strong>{status.indexed_files.toLocaleString()}</strong></div>
          <div className="side-stat"><span>Text searchable</span><strong>{progress.text_indexed.toLocaleString()}</strong></div>
          <div className="side-section-label tree-heading">REPOSITORY FILES</div>
          <nav className="file-tree" aria-label="Repository file tree">
            {renderTree("", 0)}
          </nav>
          <div className="capability-key">
            <span className="level level-zero">L0</span><span>File identified</span>
            <span className="level level-one">L1</span><span>Text searchable</span>
          </div>
          <div className="sidebar-bottom"><span className="shield">◇</span><span>Index stored in <code>.codewiki/</code></span></div>
        </aside>

        <section className="content-area">
          <div className="page-heading">
            <div><p className="eyebrow">WORKSPACE EXPLORER</p><h1>Find your way through the code</h1><p className="muted">Search files and readable text across this workspace.</p></div>
            <div className={`index-state ${progress.state}`}><span className="state-icon">{progress.state === "indexing" ? "◌" : progress.state === "failed" ? "!" : "✓"}</span><span>{progress.state === "indexing" ? `${progress.files_seen.toLocaleString()} discovered` : progress.state === "failed" ? "Indexing stopped" : "Index ready"}</span></div>
          </div>

          {progress.state === "indexing" && <div className="progress-strip"><div className="progress-track"><span /></div><span>{progress.files_indexed.toLocaleString()} indexed · {progress.text_indexed.toLocaleString()} text searchable</span></div>}
          {(error || progress.error) && <p className="error-banner" role="alert">{error ?? progress.error}</p>}
          {progress.diagnostics > 0 && <p className="diagnostic-note">{progress.diagnostics} file or traversal issue(s) were recorded. Some entries may have limited coverage.</p>}

          <details className="analysis-panel">
            <summary>Java call analysis</summary>
            <button className="analysis-refresh" type="button" onClick={() => void loadAnalysis()}>Refresh analysis</button>
            {analysis?.run ? <>
              <p className="analysis-meta">Atlas {analysis.run.provider_version} · {analysis.run.summary.indexed_files} Java files · {analysis.run.summary.symbols} symbols · {analysis.run.summary.failed_files} failed</p>
              <div className="analysis-facts">
                <div><strong>Call edges and candidates</strong>
                  {analysis.calls.length ? <ul>{analysis.calls.map((call, index) => <li key={`${call.source_name}:${call.target_name}:${index}`}><span>{call.source_name} → {call.target_name}</span><small>{call.resolution_state} · {call.evidence?.path ?? "source unavailable"}{call.evidence?.line_start ? `:${call.evidence.line_start}` : ""}</small></li>)}</ul> : <p>No call edges were returned. Atlas leaves some Java calls unresolved.</p>}
                </div>
                <div><strong>Unresolved callsites</strong>
                  {analysis.unresolved_calls.length ? <ul>{analysis.unresolved_calls.map((call, index) => <li key={`${call.evidence.path}:${call.evidence.line_start}:${index}`}><span>{call.name}</span><small>{call.unresolved_reason} {call.evidence.path}:{call.evidence.line_start ?? "?"}</small></li>)}</ul> : <p>No unresolved callsites were reported.</p>}
                </div>
              </div>
              <p className="analysis-meta">Detailed facts and the DOT call graph are saved under <code>.codewiki/analysis/</code>.</p>
            </> : <p className="analysis-meta">Run <code>codegraphy init &lt;workspace&gt;</code> to build the Java graph and source-linked reports.</p>}
          </details>

          <div className="search-wrap"><span className="search-icon" aria-hidden="true">⌕</span><input aria-label="Search files and text" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search files and readable text…" /><kbd>↵</kbd></div>

          <div className="results-layout">
            <section className="results-pane" aria-label="Files">
              <div className="results-heading"><span>{query.trim() ? "SEARCH RESULTS" : "FILES"}</span><span>{loading ? "Searching…" : `${files.length}${files.length === 100 ? "+" : ""} items`}</span></div>
              <div className="file-list">
                {files.map(({ file, snippet }) => (
                  <button className={`file-row ${selected?.id === file.id ? "selected" : ""}`} key={file.id} onClick={() => void selectFile(file)}>
                    <span className={`file-icon ${file.language ? "source-icon" : "doc-icon"}`}>{file.language ? "‹›" : "▤"}</span>
                    <span className="file-main"><span className="file-name">{file.display_name}</span><span className="file-path">{file.path}</span>{snippet && <span className="file-snippet">{snippet}</span>}</span>
                    <span className="file-meta"><span className={`level ${file.capability_level ? "level-one" : "level-zero"}`}>L{file.capability_level}</span><span>{formatBytes(file.byte_size)}</span></span>
                  </button>
                ))}
                {!loading && files.length === 0 && <div className="empty-results"><span>⌕</span><strong>{query.trim() ? "No matches found" : "No files indexed yet"}</strong><p>{query.trim() ? "Try another phrase or a shorter search." : "The catalog will appear here as files are discovered."}</p></div>}
              </div>
            </section>

            <section className="preview-pane" aria-label="Source preview">
              {selected ? (
                <>
                  <div className="preview-header"><div><span className="preview-kicker">{fileKind(selected).toUpperCase()}</span><h2>{selected.display_name}</h2><p>{selected.path}</p></div><span className={`level ${selected.capability_level ? "level-one" : "level-zero"}`}>L{selected.capability_level}</span></div>
                  <div className="preview-details"><span>{selected.detected_format.replaceAll("_", " ")}</span><span>{formatBytes(selected.byte_size)}</span><span>{selected.parse_status.replaceAll("-", " ")}</span></div>
                  {selected.diagnostic && <p className="diagnostic-note preview-diagnostic">{selected.diagnostic}</p>}
                  {selected.is_binary ? <div className="preview-empty"><span>▧</span><strong>Binary file</strong><p>This file is cataloged by metadata. A format-specific viewer is not available in this milestone.</p></div> : preview ? <div className="code-preview"><pre>{preview.content}</pre>{preview.truncated && <div className="truncated-note">Preview limited to the first 1 MiB.</div>}</div> : <div className="preview-empty"><span className="spinner" /><p>Loading source preview…</p></div>}
                </>
              ) : (
                <div className="preview-empty preview-placeholder"><span className="source-glyph">{`{ }`}</span><strong>Select a file to preview</strong><p>Source opens here with its location and catalog coverage.</p></div>
              )}
            </section>
          </div>
          <footer className="content-footer"><span>Coverage is shown per file. Cataloged files are not necessarily semantically analyzed.</span><span>Offline · No project code executed</span></footer>
        </section>
      </div>
    </main>
  );
}
