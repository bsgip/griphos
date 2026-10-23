# Installing griphos

griphos is not yet published on crates.io or PyPI, so it needs to be built from source from this git repository. The repository is currently private, so you need an SSH key registered with GitHub that has access to `bsgip/griphos`.

## Rust users

You need a Rust toolchain, which you can get from [rustup.rs](https://rustup.rs).

To add griphos to your Rust project, add this to your project's `Cargo.toml`:

```toml
[dependencies]
griphos = { git = "ssh://git@github.com/bsgip/griphos.git", branch = "main" }
# Or choose a different branch, or pin with `tag = "v0.1.0"` or `rev = "<commit sha>"`.
```

Cargo clones and builds it automatically the next time you build your project.

## Python users

griphos has Python bindings that are compiled from Rust on install. You need:

1. **A Rust toolchain.** Install it with [rustup](https://rustup.rs). Check with `rustc --version`.
2. **A C linker/compiler**, which Rust uses to link.
3. **Python and pip.** A virtual environment is recommended; we assume you have already created and activated one.

You do not need to install maturin yourself; pip fetches it automatically as the build backend.

Then install using pip:

```sh
pip install "git+ssh://git@github.com/bsgip/griphos.git@main"
```

To pin a version, replace `@main` with `@v0.1.0` (or another tag, branch, or commit SHA).

## Developing griphos locally

Clone the repo and work inside it:

```sh
git clone git@github.com:bsgip/griphos.git
cd griphos
```

**Rust:** use the normal Cargo workflow (`cargo build`, `cargo test`).

**Python bindings:** in a virtual environment, install maturin and build in editable mode:

```sh
python -m venv .venv
source .venv/bin/activate
pip install maturin
maturin develop
```

Re-run `maturin develop` after changing Rust code.

After modifying the Rust code for the Python bindings, regenerate the Python stubs:

```sh
cd python
cargo run --bin stub_gen
```

**Git hooks:** this repo has a pre-push hook that checks the Python stubs (`python/python/griphos/__init__.pyi`) are up to date. Enable it once per clone:

```sh
git config core.hooksPath .githooks
```
