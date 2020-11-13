#!/usr/bin/env bash
set -e

if [ -n "$MINIKUBE_DOCKER_ENV" ]; then
  eval $(minikube -p minikube docker-env)
fi

tmp_dir=$(mktemp -d -t fluvio-docker-image-XXXXXX)
echo "tmp_dir: ${tmp_dir}"
cp ../../target/x86_64-unknown-linux-musl/$CARGO_PROFILE/hello $tmp_dir/
cp $(dirname $0)/Dockerfile $tmp_dir/Dockerfile
cd $tmp_dir
docker build -t k8-hello:$DOCKER_TAG .
rm -rf $tmp_dir
