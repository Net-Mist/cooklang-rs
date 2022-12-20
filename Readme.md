# Cooklang-rs

A cooklang parser, implemented in Rust for Rust and Python, following [the EBNF of the language](https://github.com/cooklang/spec/blob/main/EBNF.md)

The rust parser is implemented using a parser combinator: [nom](https://docs.rs/nom/latest/nom/). Precise data structure are defined for the different element of the language : `Metadata`, `Ingredient`, `Cookware` and `Timer`.

The python parser is a binding of the rust parser using [PyO3](https://github.com/PyO3/pyo3)

## Test

Both pass the canonical tests.

To run the test in rust, run:

```sh
cd cooklang-rs
cargo test
```

To run the test in python, first install the package using

```sh
maturin develop
```

then run

```sh
python -m unittest discover -s ./tests
```
