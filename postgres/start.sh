#!/bin/sh

doas podman run -it --rm --name postgres -e POSTGRES_HOST_AUTH_METHOD=trust \
  -v ./data:/var/lib/postgresql/data \
  -p 5432:5432 \
  postgres
