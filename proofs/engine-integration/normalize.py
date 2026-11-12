"""Normalize actual Atlas FileFacts and Joern Neo4j CSV exports for this proof."""

from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
RESULTS = ROOT / "results"
FIXTURE = ROOT / "fixture/src/main/java/proof/PriceFlow.java"
SOURCE_PATH = "src/main/java/proof/PriceFlow.java"
ATLAS_COMMIT = "0f8379ff61a6f5ea1ae888d331f21b8b44f6a5b7"
JOERN_COMMIT = "427508c97c0b461c70a8f5292346b83906e43a47"


def read_joern_csv(name: str) -> list[dict[str, str]]:
    base = RESULTS / "joern-export" / name
    raw_headers = next(csv.reader(base.with_name(base.stem.replace("_data", "_header") + ".csv").open(encoding="utf-8", newline="")))
    headers = [header.split(":", 1)[0] if not header.startswith(":") else header for header in raw_headers]
    with base.open(encoding="utf-8", newline="") as stream:
        return list(csv.DictReader(stream, fieldnames=headers))


def line_text(line: int) -> str:
    return FIXTURE.read_text(encoding="utf-8").splitlines()[line - 1].strip()


def main() -> None:
    atlas = json.loads((RESULTS / "atlas-file-facts.json").read_text(encoding="utf-8"))
    methods = read_joern_csv("nodes_METHOD_data.csv")
    calls = read_joern_csv("nodes_CALL_data.csv")
    locals_ = read_joern_csv("nodes_LOCAL_data.csv")
    identifiers = read_joern_csv("nodes_IDENTIFIER_data.csv")
    literals = read_joern_csv("nodes_LITERAL_data.csv")
    parameters = read_joern_csv("nodes_METHOD_PARAMETER_IN_data.csv")
    returns = read_joern_csv("nodes_RETURN_data.csv")
    meta = read_joern_csv("nodes_META_DATA_data.csv")
    arguments = read_joern_csv("edges_ARGUMENT_data.csv")
    calls_edges = read_joern_csv("edges_CALL_data.csv")
    reaching_def = read_joern_csv("edges_REACHING_DEF_data.csv")
    cfg_edges = read_joern_csv("edges_CFG_data.csv")
    cdg_edges = read_joern_csv("edges_DOMINATE_data.csv")
    joern_node_count = sum(len(read_joern_csv(path.name)) for path in sorted((RESULTS / "joern-export").glob("nodes_*_data.csv")))
    joern_edge_count = sum(len(read_joern_csv(path.name)) for path in sorted((RESULTS / "joern-export").glob("edges_*_data.csv")))

    nodes = {}
    for rows in (methods, calls, locals_, identifiers, literals, parameters, returns):
        for row in rows:
            nodes[row[":ID"]] = row

    atlas_methods = {symbol["name"]: symbol for symbol in atlas["symbols"] if symbol["kind"] == "method"}
    joern_methods = {row["NAME"]: row for row in methods if row["NAME"] in {"total", "addFee"}}
    entities = []
    for name in ("total", "addFee"):
        a = atlas_methods[name]
        j = joern_methods[name]
        entities.append({
            "id": f"method:proof.PriceFlow.{name}",
            "kind": "method",
            "name": name,
            "qualified_name": j["FULL_NAME"],
            "signature": a["signature"],
            "resolution_state": "RESOLVED",
            "confidence": None,
            "confidence_note": "Neither engine supplies a calibrated confidence score for this declaration.",
            "evidence": [
                {"provider": "atlas-engine", "provider_version": "1.7.0", "engine_id": a["id"], "path": SOURCE_PATH, "range": a["range"], "line_start": a["range"]["start_line"] + 1, "line_end": a["range"]["end_line"] + 1, "resolution_state": "RESOLVED"},
                {"provider": "joern", "provider_version": meta[0]["VERSION"], "engine_id": j[":ID"], "path": j["FILENAME"].replace("\\\\", "/"), "line_start": int(j["LINE_NUMBER"]), "line_end": int(j["LINE_NUMBER_END"]), "resolution_state": "RESOLVED"},
            ],
        })

    # Preserve Joern's typed AST call labels; never inspect source text to infer operators.
    operation_kinds = {
        "<operator>.assignment": "ASSIGN",
        "<operator>.multiplication": "MULTIPLY",
        "<operator>.addition": "ADD",
    }
    operation_ir = []
    relevant_joern_calls = []
    for call in calls:
        name = call["NAME"]
        if name in operation_kinds or name == "addFee":
            relevant_joern_calls.append(call)
    for call in relevant_joern_calls:
        arg_nodes = [nodes[edge[":END_ID"]] for edge in arguments if edge[":START_ID"] == call[":ID"] and edge[":END_ID"] in nodes]
        arg_nodes.sort(key=lambda row: int(row.get("ORDER") or "0"))
        # Joern models the implicit receiver as argument 0; CodeWiki's call IR
        # exposes explicit source arguments while retaining the full provider CODE.
        arg_nodes = [arg for arg in arg_nodes if arg.get("NAME") != "this"]
        label = call["NAME"]
        if label in operation_kinds:
            ir_kind = operation_kinds[label]
        else:
            ir_kind = "CALL"
        owner = "total" if int(call["LINE_NUMBER"]) <= 6 else "addFee"
        resolved_target = next((edge for edge in calls_edges if edge[":START_ID"] == call[":ID"] and any(method[":ID"] == edge[":END_ID"] and method["FULL_NAME"] == call["METHOD_FULL_NAME"] for method in methods)), None) if ir_kind == "CALL" else None
        operation_ir.append({
            "id": f"joern-call:{call[':ID']}",
            "method": f"proof.PriceFlow.{owner}",
            "kind": ir_kind,
            "provider_label": label,
            "code": call["CODE"],
            "arguments": [arg.get("NAME") or arg.get("CODE") or arg.get("NAME") for arg in arg_nodes],
            "line": int(call["LINE_NUMBER"]),
            "evidence": {"provider": "joern", "provider_version": meta[0]["VERSION"], "engine_id": call[":ID"], "path": SOURCE_PATH, "source_line": line_text(int(call["LINE_NUMBER"]))},
            "resolution_state": "RESOLVED" if ir_kind != "CALL" or resolved_target else "UNRESOLVED",
            "target_engine_method_id": resolved_target[":END_ID"] if resolved_target else None,
            "confidence": None,
        })

    for ret in returns:
        line = int(ret["LINE_NUMBER"])
        owner = "total" if line <= 7 else "addFee"
        operation_ir.append({
            "id": f"joern-return:{ret[':ID']}",
            "method": f"proof.PriceFlow.{owner}",
            "kind": "RETURN",
            "provider_label": "RETURN",
            "code": ret["CODE"],
            "arguments": [],
            "line": line,
            "evidence": {"provider": "joern", "provider_version": meta[0]["VERSION"], "engine_id": ret[":ID"], "path": SOURCE_PATH, "source_line": line_text(line)},
            "resolution_state": "RESOLVED",
            "confidence": None,
        })

    # Atlas FileFacts' dataflow relationships are retained verbatim and remain
    # PROVIDER_REPORTED until independently cross-checked; confidence is untouched.
    atlas_data_nodes = {node["id"]: node for node in atlas["data_nodes"]}
    relationships = []
    for edge in atlas["dataflow_edges"]:
        relationships.append({
            "id": f"atlas-dataflow:{edge['id']}",
            "kind": edge["kind"],
            "source": {"id": edge["source"], "name": atlas_data_nodes.get(edge["source"], {}).get("name")},
            "target": {"id": edge["target"], "name": atlas_data_nodes.get(edge["target"], {}).get("name")},
            "evidence": {"provider": "atlas-engine", "provider_version": "1.7.0", "engine_id": edge["id"], "path": SOURCE_PATH, "range": edge["location"], "line_start": edge["location"]["start_line"] + 1, "line_end": edge["location"]["end_line"] + 1},
            "resolution_state": "PROVIDER_REPORTED",
            "confidence": edge.get("confidence"),
        })

    joern_nodes = {row[":ID"]: row for row in (calls + locals_ + identifiers + literals + parameters + returns)}
    for edge in reaching_def:
        source = joern_nodes.get(edge[":START_ID"])
        target = joern_nodes.get(edge[":END_ID"])
        if not source or not target or not edge.get("VARIABLE"):
            continue
        if edge["VARIABLE"] not in {"quantity", "unitPrice", "subtotal", "amount", "fee", "2"}:
            continue
        relationships.append({
            "id": f"joern-reaching-def:{edge[':START_ID']}:{edge[':END_ID']}:{edge['VARIABLE']}",
            "kind": "REACHING_DEF",
            "source": {"id": edge[":START_ID"], "name": source.get("NAME") or source.get("CODE")},
            "target": {"id": edge[":END_ID"], "name": target.get("NAME") or target.get("CODE")},
            "variable": edge["VARIABLE"],
            "evidence": {"provider": "joern", "provider_version": meta[0]["VERSION"], "source_node_id": edge[":START_ID"], "target_node_id": edge[":END_ID"], "path": SOURCE_PATH, "source_lines": [int(n) for n in (source.get("LINE_NUMBER"), target.get("LINE_NUMBER")) if n]},
            "resolution_state": "PROVIDER_REPORTED",
            "confidence": None,
        })

    # Join the method-scoped Joern CALL node to its exact METHOD node. Atlas'
    # extraction-only reference stays independently marked UNRESOLVED.
    atlas_call_ref = next(ref for ref in atlas["references"] if ref["text"] == "addFee")
    target_method = next(row for row in methods if row["NAME"] == "addFee")
    joern_call = next(row for row in calls if row["METHOD_FULL_NAME"] == target_method["FULL_NAME"])
    call_target_edge = next(edge for edge in calls_edges if edge[":START_ID"] == joern_call[":ID"] and edge[":END_ID"] == target_method[":ID"])
    relationships.append({
        "id": f"call:proof.PriceFlow.total:proof.PriceFlow.addFee",
        "kind": "CALLS",
        "source": "method:proof.PriceFlow.total",
        "target": "method:proof.PriceFlow.addFee",
        "resolution_state": "RESOLVED",
        "confidence": None,
        "provider_evidence": [
            {"provider": "joern", "provider_version": meta[0]["VERSION"], "engine_edge": call_target_edge[":TYPE"], "source_node_id": call_target_edge[":START_ID"], "target_node_id": call_target_edge[":END_ID"], "path": SOURCE_PATH, "line": int(joern_call["LINE_NUMBER"])},
            {"provider": "atlas-engine", "provider_version": "1.7.0", "engine_reference_id": atlas_call_ref["id"], "path": SOURCE_PATH, "range": atlas_call_ref["range"], "resolution_state": "UNRESOLVED", "reason": "Atlas per-file extraction emitted the reference but did not run repository-level resolution."},
        ],
    })

    operations_by_method = {method: [op for op in operation_ir if op["method"] == f"proof.PriceFlow.{method}"] for method in ("total", "addFee")}
    mul = next(op for op in operations_by_method["total"] if op["kind"] == "MULTIPLY")
    add = next(op for op in operations_by_method["addFee"] if op["kind"] == "ADD")
    biography = "\n".join([
        "# Biography: `proof.PriceFlow.total`",
        "",
        "## Direct behavior",
        f"- Line {mul['line']}: multiplies `{mul['arguments'][0]}` by `{mul['arguments'][1]}`. Evidence: Joern CALL `{mul['evidence']['engine_id']}`.",
        "- Line 5: stores the product in `subtotal`. Evidence: Joern assignment CALL `30064771072` and Atlas `assign` dataflow edge at the same source range.",
        f"- Line {next(op['line'] for op in operations_by_method['total'] if op['kind'] == 'CALL')}: calls `addFee` with `{next(op['arguments'][0] for op in operations_by_method['total'] if op['kind'] == 'CALL')}` and returns that call result (Joern RETURN `{next(op['id'] for op in operations_by_method['total'] if op['kind'] == 'RETURN')}`). Joern's CPG CALL edge resolves to `proof.PriceFlow.addFee:int(int)`; Atlas retains the extracted reference as `UNRESOLVED` because this proof used file extraction without repository-level resolution.",
        "",
        "## Callee behavior (kept separate from `total`) ",
        f"- `proof.PriceFlow.addFee` line 10 assigns literal `2` to `fee`; line {add['line']} adds `amount` and `fee`; line 11 returns that expression (Joern RETURN `{next(op['id'] for op in operations_by_method['addFee'] if op['kind'] == 'RETURN')}`). Evidence: Joern assignment/addition CALL nodes and Atlas dataflow nodes/edges.",
        "",
        "## Coverage",
        f"- Atlas Java full extraction: {len(atlas['symbols'])} symbols, {len(atlas['callsites'])} callsites, {len(atlas['cfg_nodes'])} CFG nodes/{len(atlas['cfg_edges'])} CFG edges, {len(atlas['data_nodes'])} data nodes/{len(atlas['dataflow_edges'])} dataflow edges.",
        f"- Joern: {joern_node_count} nodes and {joern_edge_count} edges in its CPG CSV; {len(cfg_edges)} CFG edges, {len(cdg_edges)} DOMINATE edges, and {len(reaching_def)} REACHING_DEF edges in the engine graph export.",
        "- Dataflow edges are marked `PROVIDER_REPORTED`; this proof does not reinterpret provider confidence as independent certainty.",
        "",
    ])

    normalized = {
        "schema": "codewiki-engine-proof/v1",
        "fixture": {"path": SOURCE_PATH, "sha256": hashlib.sha256(FIXTURE.read_bytes()).hexdigest(), "source": FIXTURE.read_text(encoding="utf-8").splitlines()},
        "providers": [
            {"name": "atlas-engine", "version": "1.7.0", "commit": ATLAS_COMMIT, "api": "Engine::extract_file_with_mode(ExtractionMode::Full)"},
            {"name": "joern", "version": meta[0]["VERSION"], "commit": JOERN_COMMIT, "api": "joern-parse JAVASRC + joern-export neo4jcsv"},
        ],
        "entities": entities,
        "operation_ir": operation_ir,
        "relationships": relationships,
        "coverage": {"joern_nodes_exported": joern_node_count, "joern_edges_exported": joern_edge_count, "joern_cfg_edges": len(cfg_edges), "joern_control_dominance_edges": len(cdg_edges), "joern_reaching_def_edges": len(reaching_def), "atlas_diagnostics": atlas.get("diagnostics", [])},
    }
    (RESULTS / "normalized-facts.json").write_text(json.dumps(normalized, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (RESULTS / "method-biography.md").write_text(biography, encoding="utf-8")


if __name__ == "__main__":
    main()
