# FRDM-MCXN947 with Rust

Tinkering with Rust and Embassy on the MCXN947

Uses the mcx-pac for now (embassy is working towards an embassy-nxp pac/hal)


Make sure you have the latest stable rust and cargo (using rustup) along with
the architecture support.

```sh
rustup update
rustup target add thumbv8m.main-none-eabihf
```

Easily run things with probe-rs, though needs a patch still.

```sh
git clone git@github.com:probe-rs/probe-rs.git
cd probe-rs
git remote add bksalman git@github.com:bksalman/probe-rs.git
git fetch bksalman
git checkout -t bksalman/mcxn947-support
cargo install --path probe-rs-tools --locked
```

Then this project is runnable with.
  
```sh
cargo run
```

