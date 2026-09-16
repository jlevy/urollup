#!/bin/sh
# Install the reviewed cargo-deny release into a directory, for CI jobs that run
# `make audit` or `make gate-proofs`.
#
# Supply-chain policy (SUPPLY-CHAIN-SECURITY.md): the version is pinned to a release at
# least 14 days old, and the archive is verified against its pinned SHA-256 digest before
# extraction. `supply-chain-policy.json` inventories this file and checks both the version
# and the digest against the GitHub release on every run. Linux x86_64 only; local
# development installs with `cargo install cargo-deny --locked --version 0.20.2`.

set -eu

CARGO_DENY_VERSION="0.20.2"
ASSET="cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz"
ASSET_SHA256="9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f"
URL="https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/${ASSET}"

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <install-directory>" >&2
    exit 2
fi
if [ "$(uname -s)" != "Linux" ] || [ "$(uname -m)" != "x86_64" ]; then
    echo "error: this installer supports Linux x86_64 only; install locally with" >&2
    echo "       cargo install cargo-deny --locked --version ${CARGO_DENY_VERSION}" >&2
    exit 2
fi

install_dir="$1"
work_dir="$(mktemp -d)"
trap 'rm -rf -- "$work_dir"' EXIT

curl --proto '=https' --tlsv1.2 -fsSL -o "$work_dir/$ASSET" "$URL"
printf '%s  %s\n' "$ASSET_SHA256" "$work_dir/$ASSET" | sha256sum --check --strict -
tar -xzf "$work_dir/$ASSET" -C "$work_dir"
mkdir -p "$install_dir"
install -m 0755 "$work_dir/cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl/cargo-deny" "$install_dir/cargo-deny"
"$install_dir/cargo-deny" --version
