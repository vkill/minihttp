# minihttp

This is a fork of [`tokio-minihttp`] ported to [`async-std`] and updated to to the
latest version of [`tokio`].

Note that [`tokio-minihttp`] is at the top of [TechEmpower benchmarks].

This benchmark compares the performance of async runtimes at serving HTTP.

## Usage

Start an [`async-std`] or [`tokio`] server by running one of the following two
commands:

* `cargo run --release --example async-std`
* `cargo run --release --example tokio`

Start a benchmark by using either [`autocannon`] or [`wrk`]:

* `autocannon 0.0.0.0:8080/plaintext`
* `wrk -t1 -c50 -d10 http://0.0.0.0:8080/plaintext`

[`tokio-minihttp`]: https://github.com/tokio-rs/tokio-minihttp
[TechEmpower benchmarks]: https://www.techempower.com/benchmarks/#section=data-r18&hw=ph&test=plaintext
[`async-std`]: https://github.com/async-rs/async-std
[`tokio`]: https://github.com/tokio-rs/tokio
[`wrk`]: https://github.com/wg/wrk
[`autocannon`]: https://github.com/mcollina/autocannon
