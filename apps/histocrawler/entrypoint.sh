#!/bin/sh
# Clear Docker Desktop / compose proxy vars that break Rust reqwest when empty or mis-set.
unset HTTP_PROXY HTTPS_PROXY http_proxy https_proxy ALL_PROXY all_proxy
export NO_PROXY='*'
export no_proxy='*'
exec /usr/src/app/histocrawler
