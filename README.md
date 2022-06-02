# LPA

A high performance parallel Label Propagation Algorithm implemented in Rust.

**LPA** (Label Propagation Algorithm) is a simple and fast community detection algorithm. The algorithm is as follow.

1. Initialize the label of every vertice with its id.
2. Traverse over each node and update its own label with the largest number of labels in the labels of its neighbor nodes.
3. Repeat step 2 until the termination conditions are met.

**Modularity** is a commonly used index to measure the quality of community partition. The modularity is calculated by the following equation.

![modularity](https://latex.codecogs.com/gif.download?Q%3D%5Csum_%7Bv%3D1%7D%5E%7Bk%7D%5B%5Cfrac%7Bl_v%7D%7BM%7D-%28%5Cfrac%7Bd_v%7D%7B2M%7D%29%5E2%5D)
$$
Q=\sum_{v=1}^{k}[\frac{l_v}{M}-(\frac{d_v}{2M})^2]
$$

## Build

```bash
cargo build --release
```

## Usage

```
USAGE:
    lpa [OPTIONS] --delimiter <DELIMITER> <CSV_EDGE_PATH>

ARGS:
    <CSV_EDGE_PATH>    

FLAGS:
    -h, --help       Print help information
    -V, --version    Print version information

OPTIONS:
    -d, --delimiter <DELIMITER>    csv delimiter [possible values: white-space, tab, comma]
    -l, --limit <LIMIT>            iteration limit [default: 20]
    -o, --output <OUTPUT>          output file
```

The code takes an input graph in a csv file. Every row indicates an edge between two nodes separated by a delimiter (white-space, tab or comma). The first row is a header (#vertices, #edges). Nodes should be indexed starting with 0.

An example input graph is as follow.

```
# comment
# The first low is a pair indicating the number of vertices and the number of edges.
4 6
0 1
1 2
2 3
3 0
0 2
1 3
```

## Reference
https://github.com/wmjtxt/LPA
