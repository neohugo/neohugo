# GitHub:       https://github.com/neohugo/neohugo
# Website:      https://neohugo.github.io/

ARG GO_VERSION="1.25"
ARG DEBIAN_VERSION="trixie"
ARG DART_SASS_VERSION="1.80.3"
ARG MOZJPEG_VERSION="4.1.1"

# Build stage
FROM golang:${GO_VERSION}-${DEBIAN_VERSION} AS build

# Build configuration
ENV HUGO_BUILD_TAGS=extended,withdeploy
ENV CGO_ENABLED=1
ENV GOOS=linux
ENV GO111MODULE=on

WORKDIR /go/src/github.com/neohugo/neohugo

COPY . /go/src/github.com/neohugo/neohugo/

# Install build dependencies and build neohugo
RUN apt-get update && \
    apt-get install -y gcc g++ libc6-dev && \
    go install github.com/magefile/mage@latest && \
    mage neohugo && \
    mage install

# Dart-sass download stage
FROM debian:${DEBIAN_VERSION} AS dart-sass
ARG DART_SASS_VERSION
ARG TARGETARCH
ARG DART_ARCH=${TARGETARCH:-amd64}
WORKDIR /out
RUN apt-get update && apt-get install -y curl && \
    ARCH=$(echo ${DART_ARCH} | sed 's/amd64/x64/') && \
    curl -L https://github.com/sass/dart-sass/releases/download/${DART_SASS_VERSION}/dart-sass-${DART_SASS_VERSION}-linux-${ARCH}.tar.gz | tar -xz

# Final stage
FROM node:24-${DEBIAN_VERSION}

ARG MOZJPEG_VERSION

LABEL description="Docker container for building static sites with the neohugo static site generator"
LABEL maintainer="Prachya Saechua<blackb1rd@blackb1rd.dev>"

# Copy neohugo binary from build stage
COPY --from=build /go/bin/neohugo /usr/bin/neohugo

# Install runtime dependencies and build mozjpeg
RUN export DEBIAN_FRONTEND=noninteractive && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    bash \
    ca-certificates \
    exiftool \
    git \
    libpng-dev \
    libstdc++6 \
    libwebp-dev \
    webp \
    openssh-client \
    pngquant \
    rsync \
    zlib1g-dev \
    autoconf \
    automake \
    binutils \
    build-essential \
    cmake \
    curl \
    g++ \
    gcc \
    gnupg \
    gzip \
    libtool \
    make \
    nasm \
    pkg-config \
    tar \
    xz-utils && \
    # Remove node user and create neohugo user
    deluser --remove-home node && \
    addgroup --system --gid 1000 neohugo && \
    adduser --system --uid 1000 --ingroup neohugo --home /src neohugo && \
    # Build and install mozjpeg
    curl -L https://github.com/mozilla/mozjpeg/archive/v${MOZJPEG_VERSION}.tar.gz | tar -xz && \
    cd mozjpeg-${MOZJPEG_VERSION} && \
    cmake -DCMAKE_BUILD_TYPE=Release . && \
    make install && cd .. && \
    rm -rf mozjpeg-${MOZJPEG_VERSION} && \
    # Clean up build dependencies
    apt-get purge -y --auto-remove \
    autoconf \
    automake \
    binutils \
    build-essential \
    cmake \
    g++ \
    gcc \
    gnupg \
    libtool \
    make \
    nasm \
    pkg-config \
    xz-utils && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    # Configure git for neohugo user
    runuser -u neohugo -- git config --global --add safe.directory /src && \
    runuser -u neohugo -- git config --global core.quotepath false && \
    # Verify installations
    /opt/mozjpeg/bin/cjpeg -version && \
    cwebp -version && \
    neohugo version

# Copy dart-sass from dart-sass stage
COPY --from=dart-sass /out/dart-sass /usr/local/bin/dart-sass

# Set up environment
ENV PATH="${PATH}:/opt/mozjpeg/bin:/usr/local/bin/dart-sass"
ENV HUGO_CACHEDIR=/cache

# Create cache directory
RUN mkdir -p /cache && chown -R neohugo:neohugo /cache

USER neohugo:neohugo
WORKDIR /src

# Expose port for live server
EXPOSE 1313

CMD ["neohugo", "--help"]
