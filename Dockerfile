# Base image for rapsberrypi 4 target
FROM rustembedded/cross:aarch64-unknown-linux-gnu

# Install pkg-config
RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes  pkg-config
