# Biography: `proof.PriceFlow.total`

## Direct behavior
- Line 5: multiplies `quantity` by `unitPrice`. Evidence: Joern CALL `30064771073`.
- Line 5: stores the product in `subtotal`. Evidence: Joern assignment CALL `30064771072` and Atlas `assign` dataflow edge at the same source range.
- Line 6: calls `addFee` with `subtotal` and returns that call result (Joern RETURN `joern-return:141733920768`). Joern's CPG CALL edge resolves to `proof.PriceFlow.addFee:int(int)`; Atlas retains the extracted reference as `UNRESOLVED` because this proof used file extraction without repository-level resolution.

## Callee behavior (kept separate from `total`) 
- `proof.PriceFlow.addFee` line 10 assigns literal `2` to `fee`; line 11 adds `amount` and `fee`; line 11 returns that expression (Joern RETURN `joern-return:141733920769`). Evidence: Joern assignment/addition CALL nodes and Atlas dataflow nodes/edges.

## Coverage
- Atlas Java full extraction: 5 symbols, 1 callsites, 8 CFG nodes/6 CFG edges, 18 data nodes/21 dataflow edges.
- Joern: 108 nodes and 368 edges in its CPG CSV; 22 CFG edges, 22 DOMINATE edges, and 77 REACHING_DEF edges in the engine graph export.
- Dataflow edges are marked `PROVIDER_REPORTED`; this proof does not reinterpret provider confidence as independent certainty.
