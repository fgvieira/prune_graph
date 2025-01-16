# Prune Graph
Fast pruning of weighted graphs with optional filtering.

## Instalation
Clone repository:
```
$ git clone https://github.com/fgvieira/prune_graph.git
```

and compile:
```
$ cargo build --release
```

To run the tests:
```
$ cargo test
```

## Usage
```
$ ./target/release/prune_graph --in input.tsv --out out.keep
```
or:
```
$ zcat input.tsv.gz | ./target/release/prune_graph > out.keep
```

To plot the graph (optional)
```
$ cat out.dot | dot -Tsvg > out.svg
```

If you want to get a full list of option, just run:
```
$ ./target/release/prune_graph --help
```

## Input data
As input, you need a `TSV` file (with or without header) with, at least, three columns. The first two columns must be the node names (defining an edge), and an additional column with the edge's weight (can be specified with `--weight_field`).

## Transform input data
If you want to transform input data, you can use any CSV manipulation tool (e.g. [Miller](https://miller.readthedocs.io/en/latest/) or [CSVtk](https://bioinf.shenwei.me/csvtk/)). For example, to use absolute values on column `5`:
```
$ cat test/example.tsv | mlr --tsv --implicit-csv-header put '$5 = abs($5)' | ./target/release/prune_graph --header [...]
```

## Filter edges
To filter edges, you can use option `--weight-filter` with any expression supported by [fasteval](https://crates.io/crates/fasteval). For example, to use column 7 as weight and only consider edges `> 0.2`:
```
$ cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_7 > 0.2" --out out.keep
```
or, if also wanted to filter on `column_3 > 1000`:
```
$ cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_3 > 1000 && column_7 > 0.2" --out out.keep
```
or, if you want to only use `0.1 < weight > 0.2`:
```
$ cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_3 > 1000 && (column_7 < 0.1 || column_7 > 0.2)" --out out.keep
```

## Output
The output will be a list of the remaining nodes after pruning. Optionally, you can also get a list of the nodes that were removed (`--out-excl`).


## Performance
Due to the way `prune_graph` is parallelized, its performance is strongly dependent on the degree of connectivity of the graph (see examples below).

<details><summary>Random function</summary>
```
shuf_seed () {
    SEED=$1; shift
    shuf --random-source <(openssl enc -aes-256-ctr -pass pass:$SEED -nosalt </dev/zero 2>/dev/null) $@
}
```
</details>

### Example 1 - Large number of components

<details><summary>Random large graph</summary>
```
$ N_NODES=10000000
$ N_EDGES=5000000
$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd
$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) | awk '{OFS="\t"; print $0,rand()}' > example_large.tsv
```
</details>

##### Mode 1
```
$ ./target/release/prune_graph -i example_large.tsv --mode 1 -v | wc -l
[2025-01-13 10:02:18.548784 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-13 10:02:18.548891 +01:00] T[main] INFO [src/main.rs:70] Reading input file "example_large.tsv"
[2025-01-13 10:02:24.710818 +01:00] T[main] INFO [src/main.rs:99] Graph has 6321958 nodes with 5000000 edges (1321959 components)
[2025-01-13 10:02:24.749254 +01:00] T[main] INFO [src/main.rs:130] Pruning heaviest position (1 threads)
[2025-01-13 10:02:59.512325 +01:00] T[main] INFO [src/main.rs:171] Pruned 2774401 nodes in 34s (79808.90 nodes/s); 3547557 nodes remaining with 12385 edges (351 components)
[2025-01-13 10:03:12.177538 +01:00] T[main] INFO [src/main.rs:185] Pruning complete in 153 iterations! Final graph has 3541739 nodes with 0 edges
[2025-01-13 10:03:12.177562 +01:00] T[main] INFO [src/main.rs:192] Saving remaining nodes
[2025-01-13 10:03:17.491300 +01:00] T[main] INFO [src/main.rs:210] Total runtime: 0.97 mins
3541739
```

##### Mode 2
```
$ ./target/release/prune_graph -i example_large.tsv --mode 2 -v 2>&1 | head -n 5
[2025-01-13 10:03:37.628890 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-13 10:03:37.628990 +01:00] T[main] INFO [src/main.rs:70] Reading input file "example_large.tsv"
[2025-01-13 10:03:44.027462 +01:00] T[main] INFO [src/main.rs:99] Graph has 6321958 nodes with 5000000 edges (1321959 components)
[2025-01-13 10:03:44.064497 +01:00] T[main] INFO [src/main.rs:130] Pruning heaviest position (1 threads)
[2025-01-13 10:13:36.945641 +01:00] T[main] INFO [src/main.rs:171] Pruned 100 nodes in 592s (0.17 nodes/s); 6321858 nodes remaining with 4999288 edges (1 components)
```

### Example 2 - Single component

<details><summary>Random 1-component graph</summary>
```
$ N_NODES=100000
$ N_EDGES=5000000
$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd
$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) | awk '{OFS="\t"; print $0,rand()}' > example_1comp.tsv
```
</details>

##### Mode 1
```
$ ./target/release/prune_graph -i example_1comp.tsv --mode 1 -v 2>&1 | head -n 5
[2025-01-13 10:25:33.833721 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-13 10:25:33.833907 +01:00] T[main] INFO [src/main.rs:70] Reading input file "example_1comp.tsv"
[2025-01-13 10:25:38.811380 +01:00] T[main] INFO [src/main.rs:99] Graph has 100000 nodes with 5000000 edges (1 components)
[2025-01-13 10:25:38.811403 +01:00] T[main] INFO [src/main.rs:130] Pruning heaviest position (1 threads)
[2025-01-13 10:30:03.933081 +01:00] T[main] INFO [src/main.rs:171] Pruned 100 nodes in 265s (0.38 nodes/s); 99900 nodes remaining with 4987002 edges (1 components)
```

##### Mode 2
```
$ ./target/release/prune_graph -i example_1comp.tsv --mode 2 -v 2>&1 | head -n 5
[2025-01-13 10:35:03.203061 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-13 10:35:03.203215 +01:00] T[main] INFO [src/main.rs:70] Reading input file "example_1comp.tsv"
[2025-01-13 10:35:09.034611 +01:00] T[main] INFO [src/main.rs:99] Graph has 100000 nodes with 5000000 edges (1 components)
[2025-01-13 10:35:09.034656 +01:00] T[main] INFO [src/main.rs:130] Pruning heaviest position (1 threads)
[2025-01-13 10:36:43.175727 +01:00] T[main] INFO [src/main.rs:171] Pruned 100 nodes in 94s (1.06 nodes/s); 99900 nodes remaining with 4987002 edges (1 components)
```

### Speed-up overview
The speed-up is measured as the average processing speed (`nodes / s`) over the first 100 iterations.

|  | Example 1 | Example 1 | Example 2 | Example 2 |
| - | - | - | - | - |
| **n_threads** | **Mode 1** | **Mode 2** | **Mode 1** | **Mode 2** |
| 1 | 79808.90 | 0.17 | 0.38 | 1.06 |
| 2 | 85185.79 | 0.20 | 0.44 | 2.11 |
| 3 | 86654.32 | 0.18 | 0.53 | 3.15 |
| 4 | 87462.56 | 0.19 | 0.51 | 3.90 |
| 5 | 89489.18 | 0.19 | 0.52 | 4.46 |
| 6 | 91585.70 | 0.18 | 0.53 | 5.34 |
| 7 | 91624.45 | 0.18 | 0.53 | 6.21 |
| 8 | 91608.91 | 0.19 | 0.55 | 6.37 |
| 9 | 91013.38 | 0.18 | 0.55 | 7.09 |
| 10 | 92407.16 | 0.18 | 0.54 | 7.89 |
| | | | | |
| 15 | 92844.73 | 0.17 | 0.56 | 9.36 |
| | | | | |
| 20 | 92003.05 | 0.18 | 0.56 | 10.52 |
