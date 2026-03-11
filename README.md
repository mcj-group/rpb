# RPB: Rust Parallel Benchmarks suite.

RPB is a benchmark suite of regular and irregular parallelism in Rust,
introduced at SPAA'24.
We invested substantial effort into developing this codebase,
and we kindly request that you acknowledge our work
by citing our paper when using this code for your own research:

[When Is Parallelism Fearless and Zero-Cost with Rust?](https://doi.org/10.1145/3626183.3659966)<br>
Javad Abdi, Gilead Posluns, Guozheng Zhang, Boxuan Wang, Mark C. Jeffrey<br>
ACM Symposium on Parallelism in Algorithms and Architectures (SPAA), 2024

# Build

## Install Rust (cargo, rustc, ...)

```bash
# download and install the version management tool: Rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# confiqure the current shell
source "$HOME/.cargo/env"
```

The `default` features are enough for benchmarking.
If you want to do more hacking, the `complete` profile could help.

## Download RPB

```bash
git clone https://github.com/mcj-group/rpb.git
cd rpb/
```

### install gcc if you don't have it on your system.

## Compile benchmarks

```bash
cargo build --release                # compile all benchmarks
cargo build --release --bin="dedup"  # compile an specific benchmark (dedup)
```

# Run
Cargo can run an individual benchmark (e.g. dedup):
```bash
cargo run --release --bin=dedup -- <input_file>
```
or the binary can be run itself:
```bash
/path/to/build/directory/pbbs/release/dedup <input_file>
```

To get the full list of flags and arguments use `--help`:
```bash
/path/to/build/directory/pbbs/release/dedup --help
```

## Inputs

All the benchmarks expect their data to be in the same format that PBBS uses.
Please check [PBBS's website](https://cmuparlay.github.io/pbbsbench/) for instruction on how to generate input data.

## Example

Run parallel dedup for 3 rounds on an input.
```bash
$ /.../dedup -o outfile -a parhash -r 3 /path/to/input

dedup:	2.560179
dedup:	2.608721
dedup:	2.492258
OutLoopTime:total:	7.966530
2.578050534s
```

# Acknowledgements

This project was inspired by the algorithms from the following sources:

- The problem-based benchmark suite: [github repo](https://github.com/cmuparlay/pbbsbench) and a [relevant paper](https://dl.acm.org/doi/10.1145/3503221.3508422) (License: [MIT](https://github.com/cmuparlay/pbbsbench/blob/master/LICENSE))
- ParlayLib: [github repo](https://github.com/cmuparlay/parlaylib) (License: [MIT](https://github.com/cmuparlay/parlaylib/blob/master/LICENSE))
