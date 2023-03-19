# !/usr/bin/env bash

docker build --build-arg LOCAL=true --tag zero2prod --file DockerfileLocal .
