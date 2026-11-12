# Joern JVM bytecode check

Joern's separately staged `jimple2cpg` frontend successfully processed the synthetic `PriceFlow.class` emitted by JDK 21 `javac` and exported a CPG. It produced 97 nodes and 419 edges, including 29 CFG, 29 DOMINATE, and 92 REACHING_DEF edges. The graph contains `total`, `addFee`, the `addFee` call edge, multiplication, addition, and assignment calls.

The `total` and `addFee` method bodies in [`joern-export/nodes_METHOD_data.csv`](joern-export/nodes_METHOD_data.csv) are rendered as Jimple from bytecode. This proves the class-file ingestion path works for this fixture. Bytecode does not preserve all source distinctions: the local `fee` and its assignment are folded into `amount + 2` in the decompiled body. Reported method lines are from class debug metadata/Jimple output and must not be treated as reliable original-source ranges without independent source mapping. Joern also emits a synthetic constructor and external operator stubs.

CSV `FILENAME` values that contained Joern's random temporary extraction directory are normalized to `${JOERN_TEMP}`; the graph facts are unchanged. The generated class file is retained in [`classes/proof/PriceFlow.class`](classes/proof/PriceFlow.class). The CPG binary is omitted; the CSV export is the review artifact.
