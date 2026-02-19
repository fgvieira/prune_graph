# Prune Graph
Fast pruning of weighted graphs with optional filtering.

## Instalation
Clone repository:
```bash
$ git clone https://github.com/fgvieira/prune_graph.git
```

and compile:
```bash
$ cargo build --release
```

To run the tests:
```bash
$ cargo test
```

## Usage
```bash
$ ./target/release/prune_graph --in input.tsv --out out.keep
```
or:
```bash
$ zcat input.tsv.gz | ./target/release/prune_graph > out.keep
```

To plot the graph (optional)
```bash
$ cat out.dot | dot -Tsvg > out.svg
```

If you want to get a full list of option, just run:
```bash
$ ./target/release/prune_graph --help
```

## Input data
As input, you need a `TSV` file (with or without header) with, at least, three columns. The first two columns must be the node names (defining an edge), and an additional column with the edge's weight (can be specified with `--weight_field`).

## Transform input data
If you want to transform input data, you can use any CSV manipulation tool (e.g. [Miller](https://miller.readthedocs.io/en/latest/) or [CSVtk](https://bioinf.shenwei.me/csvtk/)). For example, to use absolute values on column `5`:
```bash
$ cat test/example.tsv | mlr --tsv --implicit-csv-header put '$5 = abs($5)' | ./target/release/prune_graph --header [...]
```

## Filter edges
To filter edges, you can use option `--weight-filter` with any expression supported by [fasteval](https://crates.io/crates/fasteval). For example, to use column 7 as weight and only consider edges `> 0.2`:
```bash
$ cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_7 > 0.2" --out out.keep
```
or, if also wanted to filter on `column_3 > 1000`:
```bash
$ cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_3 > 1000 && column_7 > 0.2" --out out.keep
```
or, if you want to only use `0.1 < weight > 0.2`:
```bash
$ cat test/example.tsv | ./target/release/prune_graph --weight-field "column_7" --weight-filter "column_3 > 1000 && (column_7 < 0.1 || column_7 > 0.2)" --out out.keep
```

## Output
The output will be a list of the remaining nodes after pruning. Optionally, you can also get a list of the nodes that were removed (`--out-excl`).


## Performance
Due to the way `prune_graph` is parallelized, its performance is strongly dependent on the degree of connectivity of the graph (see examples below).

<details><summary>Random function</summary>

```bash
shuf_seed () {
    SEED=$1; shift
    shuf --random-source <(openssl enc -aes-256-ctr -pass pass:$SEED -nosalt </dev/zero 2>/dev/null) $@
}
```
</details>

### Example 1 - Single component, highly connected

Graph with fewer nodes than edges.

<details><summary>Code</summary>

```bash
$ N_NODES=100000
$ N_EDGES=5000000
$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd
$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) | awk '{OFS="\t"; print $0,rand()}' > example_1compH.tsv
```
</details>

### Example 2 - Single component, loosely connected

Graph similar to LD in a chromosome, where each node/snp has (on average) `$AVG_N_FWD_EDGES` edges with downstream node/snp(s).

<details><summary>Code</summary>

```bash
$ N_NODES=1000000
$ AVG_N_FWD_EDGES=10
$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" | perl -se 'srand(12345); @n = <>; chomp(@n); for ($i=0; $i <= $#n; $i++){ for ($j=$i+1; $j <= $#n; $j++) {print(join("\t", @{[$n[$i], $n[$j], sprintf("%.3f", rand())]})."\n"); last if rand() < 1/$n}}' -- -n=$AVG_N_FWD_EDGES > example_1compL.tsv
```
</details>

### Example 3 - Large number of components

Graph with more nodes than edges.

<details><summary>Code</summary>

```bash
$ N_NODES=10000000
$ N_EDGES=5000000
$ seq --equal-width 1 $N_NODES | xargs printf "node_%s\n" > /tmp/nodes.rnd
$ paste <(shuf_seed 123 --repeat --head-count $N_EDGES /tmp/nodes.rnd) <(shuf_seed 456 --repeat --head-count $N_EDGES /tmp/nodes.rnd) | awk '{OFS="\t"; print $0,rand()}' > example_large.tsv
```
</details>


### Speed-up overview
The speed-up is measured as the average processing speed (`nodes / s`) over the first 20 iterations, using default settings (unless specified otherwise).

|  | Example 1 | Example 1 | Example 2 | Example 2 | Example 3 | Example 3 |
| - | - | - | - | - | - | - |
| **n_threads** | **Mode 1** | **Mode 2** | **Mode 1** | **Mode 2** | **Mode 1** | **Mode 2** |
| 1 | 0.45 | 1.37 | 1.20 | 1.79 | 159473.62 | 0.18 |
| 2 | 0.46 | 2.05 | 1.24 | 1.90 | 167429.55 | 0.18 |
| 4 | 0.51 | 3.78 | 1.28 | 2.03 | 187083.92 | 0.18 |
| 6 | 0.54 | 5.48 | 1.28 | 2.02 | 193490.72 | 0.18 |
| 8 | 0.55 | 6.75 | 1.31 | 2.02 | 195761.61 | 0.18 |
| 10 | 0.55 | 8.13 | 1.32 | 2.03 | 199361.19 | 0.19 |
| 15 | 0.56 | 11.03 | 1.30 | 2.05 | 207449.42 | 0.19 |
| 20 | 0.57 | 12.95 | 1.31 | 2.12 | 204722.47 | 0.18 |
