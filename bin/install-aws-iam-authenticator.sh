#!/usr/bin/env bash
#
# Install the aws-iam-authenticator binary used by the EKS exec-plugin
# kubeconfig that bin/eks-up.sh emits. Pins a version, downloads to a
# file, verifies the SHA-256 checksum, then installs. Same script is
# invoked from the Dockerfile (web service) and from render.yaml's
# buildCommand (worker service on the Render Node runtime).
#
# Usage:
#   bin/install-aws-iam-authenticator.sh [install_dir]
#
# install_dir defaults to /usr/local/bin when writable, else ./bin in the
# current working directory (the Render Node runtime is non-root and ./bin
# survives between buildCommand and startCommand).
#
# Exit non-zero on any download / checksum failure so a poisoned mirror or
# truncated download can't ride out to production.

set -euo pipefail

AWS_IAM_AUTH_VERSION="${AWS_IAM_AUTH_VERSION:-0.7.15}"

# Linux/amd64 is what Render hosts and what node:20-alpine targets by default.
# Linux/arm64 is included so the Dockerfile keeps working under
# `docker build --platform=linux/arm64` on Apple Silicon.
SHA256_linux_amd64="3c7872017e02132325374ebfaf01b9abcc2f93b42b7dfc1caef0f8d48433bb34"
SHA256_linux_arm64="d75d09920a3037f1f3d7d25e10c2ad20c29f6fcfcd3abd3db8d33bd1b1c21b2d"

uname_arch=$(uname -m)
case "$uname_arch" in
  x86_64|amd64) arch=amd64; sha=$SHA256_linux_amd64 ;;
  aarch64|arm64) arch=arm64; sha=$SHA256_linux_arm64 ;;
  *)
    printf "install-aws-iam-authenticator: unsupported arch %s\n" "$uname_arch" >&2
    exit 1
    ;;
esac

install_dir="${1:-}"
if [ -z "$install_dir" ]; then
  if [ -w /usr/local/bin ] 2>/dev/null; then
    install_dir=/usr/local/bin
  else
    install_dir="$(pwd)/bin"
  fi
fi
mkdir -p "$install_dir"

url="https://github.com/kubernetes-sigs/aws-iam-authenticator/releases/download/v${AWS_IAM_AUTH_VERSION}/aws-iam-authenticator_${AWS_IAM_AUTH_VERSION}_linux_${arch}"
target="$install_dir/aws-iam-authenticator"

printf "install-aws-iam-authenticator: downloading %s\n" "$url" >&2
curl -fsSL "$url" -o "$target.tmp"

printf "%s  %s\n" "$sha" "$target.tmp" | sha256sum -c -

chmod 0755 "$target.tmp"
mv "$target.tmp" "$target"

printf "install-aws-iam-authenticator: installed %s (v%s, %s)\n" \
  "$target" "$AWS_IAM_AUTH_VERSION" "$arch" >&2
