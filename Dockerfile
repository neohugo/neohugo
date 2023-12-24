# GitHub:       https://github.com/neohugo/neohugo
# Website:      https://neohugo.github.io/

FROM golang:1.21-alpine AS build

# Optionally set HUGO_BUILD_TAGS to "extended" or "nodeploy" when building like so:
#   docker build --build-arg HUGO_BUILD_TAGS=extended .
ENV HUGO_BUILD_TAGS=extended

ARG CGO=1
ENV CGO_ENABLED=${CGO}
ENV GOOS=linux
ENV GO111MODULE=on

WORKDIR /go/src/github.com/neohugo/neohugo

COPY . /go/src/github.com/neohugo/neohugo/

# gcc/g++ are required to build SASS libraries for extended version
RUN apk update && \
    apk add --no-cache gcc g++ musl-dev git && \
    go install github.com/magefile/mage@latest

RUN mage neohugo && mage install

# ---

FROM alpine:3.18

COPY --from=build /go/bin/neohugo /usr/bin/neohugo

# libc6-compat & libstdc++ are required for extended SASS libraries
# ca-certificates are required to fetch outside resources (like Twitter oEmbeds)
RUN apk update && \
    apk add --no-cache \
      ca-certificates \
      libc6-compat \
      libstdc++ \
    && neohugo version

WORKDIR /src
# Expose port for live server
EXPOSE 1313
