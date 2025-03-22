# ------------------------------------------------------------------------------
# Environment
# ------------------------------------------------------------------------------

export CNI_NAME = podtunnel-cni
export CNI_TYPE = $(CNI_NAME)
export CNI_VERSION ?= 0.3.1

BUILD_TARGET ?= x86_64-unknown-linux-musl

CNI_BINDIR ?= /opt/cni/bin
CNI_CONFDIR ?= /etc/cni/net.d
CNI_PRIORITY ?= 99

KIND_CLUSTER ?= kind
KIND_CLUSTER_CONTAINER ?= $(KIND_CLUSTER)-control-plane
KIND_CONTAINER_RUNTIME ?= podman

# ------------------------------------------------------------------------------
# Build
# ------------------------------------------------------------------------------

.PHONY: all
all: build

.PHONY: clean
clean:
	cargo clean

.PHONY: build
build:
	cargo build --target $(BUILD_TARGET)

.PHONY: build.release
build.release:
	cargo build --target $(BUILD_TARGET) --release

# ------------------------------------------------------------------------------
# Generators
# ------------------------------------------------------------------------------

.PHONY: generate.cni.config
generate.cni.config:
	cargo xtask generate-cni-config > $(CNI_NAME).conf

.PHONY: generate.crds
generate.crds:
	cargo xtask generate-crds

# ------------------------------------------------------------------------------
# Kubernetes In Docker (KIND) - Development & Testing
# ------------------------------------------------------------------------------

.PHONY: clean.kind
clean.kind:
	kubectl --context kind-$(KIND_CLUSTER) delete crd wireguardaddresspools.podtunnel.com --ignore-not-found --wait
	kubectl --context kind-$(KIND_CLUSTER) delete crd wireguardconfigs.podtunnel.com --ignore-not-found --wait
	$(KIND_CONTAINER_RUNTIME) exec -it $(KIND_CLUSTER_CONTAINER) /bin/bash -c "rm -f /tmp/podtunnel*"
	$(KIND_CONTAINER_RUNTIME) exec -it $(KIND_CLUSTER_CONTAINER) /bin/bash -c "rm -f $(CNI_BINDIR)/$(CNI_NAME)"
	$(KIND_CONTAINER_RUNTIME) cp $(KIND_CLUSTER_CONTAINER):$(CNI_CONFDIR)/10-kindnet.conflist kindnet.conf
	jq '(.plugins) |= map(select(.type != "$(CNI_NAME)"))' kindnet.conf > updated-kindnet.conf
	$(KIND_CONTAINER_RUNTIME) cp updated-kindnet.conf $(KIND_CLUSTER_CONTAINER):$(CNI_CONFDIR)/10-kindnet.conflist
	rm -f updated-kindnet.conf kindnet.conf

.PHONY: configure.kind
configure.kind: generate.crds
	@$(KIND_CONTAINER_RUNTIME) exec -it $(KIND_CLUSTER_CONTAINER) \
		/bin/bash -c "if [ ! -f /tmp/apt.install.wg.lock ]; then apt-get update && \
			apt-get install --no-install-recommends iproute2 wireguard-tools -yq; \
			touch /tmp/apt.install.wg.lock; else true; fi"
	kubectl kustomize config/crds | kubectl --context kind-$(KIND_CLUSTER) apply -f -

.PHONY: deploy.kind
deploy.kind: build configure.kind
	$(KIND_CONTAINER_RUNTIME) cp target/$(BUILD_TARGET)/debug/$(CNI_NAME) $(KIND_CLUSTER_CONTAINER):$(CNI_BINDIR)/$(CNI_NAME)
	$(KIND_CONTAINER_RUNTIME) cp $(KIND_CLUSTER_CONTAINER):$(CNI_CONFDIR)/10-kindnet.conflist kindnet.conf
	jq 'if (.plugins | any(.type=="$(CNI_NAME)")) then . else .plugins += [{"type":"$(CNI_NAME)"}] end' kindnet.conf > updated-kindnet.conf
	$(KIND_CONTAINER_RUNTIME) cp updated-kindnet.conf $(KIND_CLUSTER_CONTAINER):$(CNI_CONFDIR)/10-kindnet.conflist
	rm -f updated-kindnet.conf kindnet.conf
