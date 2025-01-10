# Prune Graph
Fast pruning of weighted graphs with optional filtering.

### Instalation
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

### Usage
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

### Input data
As input, you need a `TSV` file (with or without header) with, at least, three columns. The first two columns must be the node names (defining an edge), and an additional column with the edge's weight (can be specified with `--weight_field`).

### Transform input data
If you want to transform input data, you can use any CSV manipulation tool (e.g. [Miller](https://miller.readthedocs.io/en/latest/) or [CSVtk](https://bioinf.shenwei.me/csvtk/)). For example, to use absolute values on column `5`:
```
$ cat test/example.tsv | mlr --tsv --implicit-csv-header put '$5 = abs($5)' | ./target/release/prune_graph --header [...]
```

### Filter edges
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

### Output
The output will be a list of the remaining nodes after pruning. Optionally, you can also get a list of the nodes that were removed (`--out-excl`).


### Performance
```
shuf_seed () {
    SEED=$1; shift
    shuf --random-source <(openssl enc -aes-256-ctr -pass pass:$SEED -nosalt </dev/zero 2>/dev/null) $@
}
```

#### Large number of components
```
$ N_NODES=10000000

$ N_EDGES=5000000

$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd

$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 789 --repeat --input-range 1-250000 --head-count $N_EDGES) | awk '{OFS="\t"; print $0,rand(),rand()}' > example_large.tsv

$ ./target/release/prune_graph -i example_large.tsv --mode 1 --weight-field column_5 -v | wc -l
[2025-01-10 12:53:12.707320 +01:00] T[main] INFO [src/main.rs:39] prune_graph v0.3.3
[2025-01-10 12:53:12.707443 +01:00] T[main] INFO [src/main.rs:51] Creating graph...
[2025-01-10 12:53:12.707449 +01:00] T[main] INFO [src/main.rs:53] Reading from input file...
[2025-01-10 12:53:21.150591 +01:00] T[main] INFO [src/main.rs:82] Graph has 6321958 nodes with 5000000 edges (1321959 components).
[2025-01-10 12:53:21.199138 +01:00] T[main] INFO [src/main.rs:110] Pruning heaviest position...
[2025-01-10 12:58:44.912707 +01:00] T[main] INFO [src/main.rs:151] Pruned 50 nodes in 323s (0.15 nodes/s); 6321908 nodes remaining with 4999633 edges (1 components).
[2025-01-10 13:04:12.392450 +01:00] T[main] INFO [src/main.rs:151] Pruned 50 nodes in 327s (0.15 nodes/s); 6321858 nodes remaining with 4999288 edges (1 components).
[...]

$ ./target/release/prune_graph -i example_large.tsv --mode 2 --weight-field column_5 -v | wc -l
[2025-01-10 11:22:49.190500 +01:00] T[main] INFO [src/main.rs:39] prune_graph v0.3.3
[2025-01-10 11:22:49.190646 +01:00] T[main] INFO [src/main.rs:51] Creating graph...
[2025-01-10 11:22:49.190654 +01:00] T[main] INFO [src/main.rs:53] Reading from input file...
[2025-01-10 11:22:58.359099 +01:00] T[main] INFO [src/main.rs:82] Graph has 6321958 nodes with 5000000 edges (1321959 components).
[2025-01-10 11:22:58.397518 +01:00] T[main] INFO [src/main.rs:110] Pruning heaviest position...
[2025-01-10 11:23:21.153921 +01:00] T[main] INFO [src/main.rs:147] Pruned 2755917 nodes in 22s (121105.22 nodes/s); 3566041 nodes remaining with 52827 edges (737 components).
[2025-01-10 11:23:34.652938 +01:00] T[main] INFO [src/main.rs:147] Pruned 17727 nodes in 13s (1313.21 nodes/s); 3548314 nodes remaining with 13938 edges (206 components).
[2025-01-10 11:23:46.777152 +01:00] T[main] INFO [src/main.rs:147] Pruned 6446 nodes in 12s (531.66 nodes/s); 3541868 nodes remaining with 2 edges (1 components).
[2025-01-10 11:23:47.012129 +01:00] T[main] INFO [src/main.rs:161] Pruning complete in 151 iterations! Final graph has 3541867 nodes with 0 edges
[2025-01-10 11:23:47.012148 +01:00] T[main] INFO [src/main.rs:168] Saving remaining nodes...
[2025-01-10 11:23:53.000206 +01:00] T[main] INFO [src/main.rs:186] Total runtime: 1.05 mins
3541867
```


#### Single component
```
$ N_NODES=100000

$ N_EDGES=5000000

$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd

$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 789 --repeat --input-range 1-250000 --head-count $N_EDGES) | awk '{OFS="\t"; print $0,rand(),rand()}' > example_1comp.tsv

$ ./target/release/prune_graph -i example_1comp.tsv --mode 1 --weight-field column_5 -v | wc -l
[2025-01-10 11:26:32.640207 +01:00] T[main] INFO [src/main.rs:39] prune_graph v0.3.3
[2025-01-10 11:26:32.640345 +01:00] T[main] INFO [src/main.rs:51] Creating graph...
[2025-01-10 11:26:32.640354 +01:00] T[main] INFO [src/main.rs:53] Reading from input file...
[2025-01-10 11:26:40.236347 +01:00] T[main] INFO [src/main.rs:82] Graph has 100000 nodes with 5000000 edges (1 components).
[2025-01-10 11:26:40.236374 +01:00] T[main] INFO [src/main.rs:110] Pruning heaviest position...
[2025-01-10 11:28:57.498736 +01:00] T[main] INFO [src/main.rs:147] Pruned 50 nodes in 137s (0.36 nodes/s); 99950 nodes remaining with 4993312 edges (1 components).
[2025-01-10 11:31:15.431371 +01:00] T[main] INFO [src/main.rs:147] Pruned 50 nodes in 137s (0.36 nodes/s); 99900 nodes remaining with 4986896 edges (1 components).
[...]

$ ./target/release/prune_graph -i example_1comp.tsv --mode 2 --weight-field column_5 -v | wc -l
[2025-01-10 13:07:01.057564 +01:00] T[main] INFO [src/main.rs:39] prune_graph v0.3.3
[2025-01-10 13:07:01.057721 +01:00] T[main] INFO [src/main.rs:51] Creating graph...
[2025-01-10 13:07:01.057730 +01:00] T[main] INFO [src/main.rs:53] Reading from input file...
[2025-01-10 13:07:08.832219 +01:00] T[main] INFO [src/main.rs:82] Graph has 100000 nodes with 5000000 edges (1 components).
[2025-01-10 13:07:08.832252 +01:00] T[main] INFO [src/main.rs:110] Pruning heaviest position...
[2025-01-10 13:09:28.125425 +01:00] T[main] INFO [src/main.rs:151] Pruned 50 nodes in 139s (0.36 nodes/s); 99950 nodes remaining with 4993312 edges (1 components).
[2025-01-10 13:11:46.974630 +01:00] T[main] INFO [src/main.rs:151] Pruned 50 nodes in 138s (0.36 nodes/s); 99900 nodes remaining with 4986896 edges (1 components).
[...]
```
