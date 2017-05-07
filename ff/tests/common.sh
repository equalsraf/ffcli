#!/bin/bash
set -ex

trap 'echo "Test failed"' ERR
export FF_PORT=$(ff start)
trap 'ff quit' EXIT

