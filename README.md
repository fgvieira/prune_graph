# Prune Graph
Fast pruning of weighted graphs with optional filtering.

### Instalation
Clone repository:
```
git clone https://github.com/fgvieira/prune_graph.git
```

and compile:
```
cargo build --release
```

To run the tests:
```
cargo test
```

### Usage
```
./target/release/prune_graph --in input.tsv --out out.keep
```
or:
```
zcat input.tsv.gz | ./target/release/prune_graph > out.keep
```

To plot the graph (optional)
```
cat out.dot | dot -Tsvg > out.svg
```

If you want to get a full list of option, just run:
```
./target/release/prune_graph --help
```

### Input data
As input, you need a `TSV` file (with or without header) with, at least, three columns. The first two columns must be the node names (defining an edge), and an additional column with the edge's weight (can be specified with `--weight_field`).

### Transform input data
If you want to transform input data, you can use any CSV manipulation tool (e.g. [Miller](https://miller.readthedocs.io/en/latest/) or [CSVtk](https://bioinf.shenwei.me/csvtk/)). For example, to use absolute values on column `5`:
```
cat test/example.tsv | mlr --tsv --implicit-csv-header put '$5 = abs($5)' | ./target/release/prune_graph --header [...]
```

### Filter edges
To filter edges, you can use option `--weight-filter` with any expression supported by [fasteval](https://crates.io/crates/fasteval). For example, to use column 7 as weight and only consider edges `> 0.2`:
```
cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_7 > 0.2" --out out.keep
```
or, if also wanted to filter on `column_3 > 1000`:
```
cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_3 > 1000 && column_7 > 0.2" --out out.keep
```
or, if you want to only use `0.1 < weight > 0.2`:
```
cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_3 > 1000 && (column_7 < 0.1 || column_7 > 0.2)" --out out.keep
```

### Output
The output will be a list of the remaining nodes after pruning. Optionally, you can also get a list of the nodes that were removed (`--out-excl`).
