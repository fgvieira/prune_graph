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
Due to the way `prune_graph` parallelizes prunning, its performance is strongly dependent on the degree of connectivity of the graph (see examples below).

```
shuf_seed () {
    SEED=$1; shift
    shuf --random-source <(openssl enc -aes-256-ctr -pass pass:$SEED -nosalt </dev/zero 2>/dev/null) $@
}
```

### Example 1 - Large number of components
```
$ N_NODES=10000000
$ N_EDGES=5000000
$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd
$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 789 --repeat --input-range 1-250000 --head-count $N_EDGES) | awk '{OFS="\t"; print $0,rand(),rand()}' > example_large.tsv
```

##### Mode 1
```
$ ./target/release/prune_graph -i example_large.tsv --mode 1 --weight-field column_5 -v | wc -l
[2025-01-10 15:40:26.840112 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-10 15:40:26.840262 +01:00] T[main] INFO [src/main.rs:69] Reading input file "example_large.tsv"
[2025-01-10 15:40:35.254778 +01:00] T[main] INFO [src/main.rs:98] Graph has 6321958 nodes with 5000000 edges (1321959 components)
[2025-01-10 15:40:35.295880 +01:00] T[main] INFO [src/main.rs:126] Pruning heaviest position (1 threads)
[2025-01-10 15:46:09.181833 +01:00] T[main] INFO [src/main.rs:167] Pruned 50 nodes in 333s (0.15 nodes/s); 6321908 nodes remaining with 4999633 edges (1 components)
[2025-01-10 15:52:00.792564 +01:00] T[main] INFO [src/main.rs:167] Pruned 50 nodes in 351s (0.14 nodes/s); 6321858 nodes remaining with 4999288 edges (1 components)
[...]
```

##### Mode 2
```
$ ./target/release/prune_graph -i example_large.tsv --mode 2 --weight-field column_5 -v | wc -l
[2025-01-10 15:43:34.998053 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-10 15:43:34.998253 +01:00] T[main] INFO [src/main.rs:69] Reading input file "example_large.tsv"
[2025-01-10 15:43:43.749941 +01:00] T[main] INFO [src/main.rs:98] Graph has 6321958 nodes with 5000000 edges (1321959 components)
[2025-01-10 15:43:43.792413 +01:00] T[main] INFO [src/main.rs:126] Pruning heaviest position (1 threads)
[2025-01-10 15:44:07.167626 +01:00] T[main] INFO [src/main.rs:167] Pruned 2755917 nodes in 23s (117899.23 nodes/s); 3566041 nodes remaining with 52827 edges (737 components)
[2025-01-10 15:44:20.872425 +01:00] T[main] INFO [src/main.rs:167] Pruned 17727 nodes in 13s (1293.51 nodes/s); 3548314 nodes remaining with 13938 edges (206 components)
[2025-01-10 15:44:33.502389 +01:00] T[main] INFO [src/main.rs:167] Pruned 6446 nodes in 12s (510.38 nodes/s); 3541868 nodes remaining with 2 edges (1 components)
[2025-01-10 15:44:33.764939 +01:00] T[main] INFO [src/main.rs:181] Pruning complete in 151 iterations! Final graph has 3541867 nodes with 0 edges
[2025-01-10 15:44:33.764963 +01:00] T[main] INFO [src/main.rs:188] Saving remaining nodes
[2025-01-10 15:44:40.309021 +01:00] T[main] INFO [src/main.rs:206] Total runtime: 1.08 mins
3541867
```

### Example 2 - Single component
```
$ N_NODES=100000
$ N_EDGES=5000000
$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd
$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 789 --repeat --input-range 1-250000 --head-count $N_EDGES) | awk '{OFS="\t"; print $0,rand(),rand()}' > example_1comp.tsv
```

##### Mode 1
```
$ ./target/release/prune_graph -i example_1comp.tsv --mode 1 --weight-field column_5 -v | wc -l
[2025-01-10 15:36:06.894498 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-10 15:36:06.894672 +01:00] T[main] INFO [src/main.rs:69] Reading input file "example_1comp.tsv"
[2025-01-10 15:36:13.109857 +01:00] T[main] INFO [src/main.rs:98] Graph has 100000 nodes with 5000000 edges (1 components)
[2025-01-10 15:36:13.109882 +01:00] T[main] INFO [src/main.rs:126] Pruning heaviest position (1 threads)
[2025-01-10 15:36:58.235908 +01:00] T[main] INFO [src/main.rs:167] Pruned 50 nodes in 45s (1.11 nodes/s); 99950 nodes remaining with 4993312 edges (1 components)
[2025-01-10 15:37:43.370222 +01:00] T[main] INFO [src/main.rs:167] Pruned 50 nodes in 45s (1.11 nodes/s); 99900 nodes remaining with 4986896 edges (1 components)
[...]
```

##### Mode 2
```
$ ./target/release/prune_graph -i example_1comp.tsv --mode 2 --weight-field column_5 -v | wc -l
[2025-01-10 15:37:59.649537 +01:00] T[main] INFO [src/main.rs:43] prune_graph v0.3.4
[2025-01-10 15:37:59.649641 +01:00] T[main] INFO [src/main.rs:69] Reading input file "example_1comp.tsv"
[2025-01-10 15:38:07.339178 +01:00] T[main] INFO [src/main.rs:98] Graph has 100000 nodes with 5000000 edges (1 components)
[2025-01-10 15:38:07.339227 +01:00] T[main] INFO [src/main.rs:126] Pruning heaviest position (1 threads)
[2025-01-10 15:40:26.924916 +01:00] T[main] INFO [src/main.rs:167] Pruned 50 nodes in 139s (0.36 nodes/s); 99950 nodes remaining with 4993312 edges (1 components)
[2025-01-10 15:42:51.856912 +01:00] T[main] INFO [src/main.rs:167] Pruned 50 nodes in 144s (0.34 nodes/s); 99900 nodes remaining with 4986896 edges (1 components)
[...]
```

### Speed-up overview (nodes / s)

|  | Example 1 | Example 1 | Example 2 | Example 2 |
| - | - | - | - | - |
| **n_threads** | **Mode 1** | **Mode 2** | **Mode 1** | **Mode 2** |
| 1 | 0.15 | 117899.23 | 1.11 | 0.36 |
| 2 | | 138428.42 | 1.90 | |
| 3 | | 139283.30 | 2.76 | |
| 4 | | 139889.86 | 3.62 | |
| 5 | | 142857.89 | 3.78 | |
| 6 | | 145813.62 | 4.75 | |
| 7 | | 143893.53 | 5.61 | |
| 8 | | 146412.25 | 6.42 | |
| 9 | | 145626.11 | 7.20 | |
| 10 | | 148060.50 | 7.02 | |
| | | | | |
| 15 | | 144732.75 | 8.36 | |
| | | | | |
| 20 | | 149349.66 | 8.58 | |
