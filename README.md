# Prune Graph
Fast graph pruning.

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
./target/release/prune_graph --in input.tsv --weight_field 7 --weight-min 0.2 --weight-type a --out-graph out.dot --out out.keep --out-excl out.pruned
```
or:
```
zcat input.tsv.gz | ./target/release/prune_graph --weight_field 7 --weight-min 0.2 --weight-type a --out-graph out.dot --out out.keep --out-excl out.pruned
```
or, to remove some edges while keeping the nodes:
```
awk '$6>1000{$7="NA"}' input.tsv | ./target/release/prune_graph --weight_field 7 --weight-min 0.2 --weight-type a --out-graph out.dot --out out.keep --out-excl out.pruned
```

To plot the graph (optional)
```
cat out.dot | dot -Tsvg > out.svg
```

If you want to get a full list of option, just run:
```
./target/release/prune_graph -h
```

### Input data
As input, you need a `TSV` file (with or without header) with, at least, three columns. The first two columns must be the node names (defining an edge), and the additional column must have the edge's weight.

### Output
The output will be a list of the remaining nodes after pruning. Optionally, you can also get a list of the files that were removed (`--out-excl`).
