# Development

## Prerequisites

* `make`
* [Rust] (using [Rustup])
* [Podman] (or [Docker])
* [Kubernetes In Docker (KIND)]
* [Kubectl]

[Rust]:https://www.rust-lang.org/
[Rustup]:https://github.com/rust-lang/rustup
[Podman]:https://github.com/containers/podman
[Docker]:https://github.com/moby/moby
[Kubernetes In Docker (KIND)]:https://github.com/kubernetes-sigs/kind
[Kubectl]:https://github.com/kubernetes/kubectl

## Environment Setup

Once you've cloned this repository you'll need to ensure you have all the
Rust components set up correctly:

```console
$ cd podtunnel/
$ rustup target add x86_64-unknown-linux-musl
```

> **Note**: The `x86_64-unknown-linux-musl` target is used for the CNI plugin
> to create statically linked binaries.

Create a Kubernetes cluster with [Kind] for development and testing:

```console
$ kind create cluster
$ kubectl wait --for=condition=Ready pods --all -A --timeout=300s
```

> **Warning**: At the time of writing, there was a prevalent [bug in kind]
> affecting many versions which would cause Pod networking to fail. [I patched
> this], but at least as I'm writing this there is still no release containing
> the patch.
>
> One easy way to identify if you are effected is to see if the
> `local-path-provisioner` is failing to deploy. If affected, use the
> workaround provided in the issue to set global `arp_ignore=0`.

The cluster is now ready for development and testing.

[Kind]:https://github.com/kubernetes-sigs/kind
[bug in kind]:https://github.com/kubernetes-sigs/kind/issues/3880
[I patched this]:https://github.com/kubernetes-sigs/kind/pull/3881

## Development Cycle

The `Makefile` will enable you to rapidly push and test changes to the cluster.

> **Note**: Some of the integration tests will create a [Kind] cluster for test
> runs as well.

You can build and deploy all required components to the cluster with:

```console
$ make deploy.kind
```

This will:

* compile the CNI and operator binaries
* generate the CRDs
* deploy everything to the Kind cluster

Then you can run some of the `configs/examples/` or otherwise testing.

You can clean everything up with:

```console
$ make clean.kind
```

[Kind]:https://github.com/kubernetes-sigs/kind

## Running Tests

To run unit and documentation tests:

```console
$ make test
```

Integration tests will deploy a `Kind` cluster and so are a little heavier
weight. They are run with:

```console
$ make test.integration
```

> **Note**: Integration tests are flagged with the `#[ignore]` attribute so
> if you're trying to run them with `cargo` directly, you'll need to flag them
> like: `cargo test -vv -- --nocapture --ignored`.

[Kind]:https://github.com/kubernetes-sigs/kind
