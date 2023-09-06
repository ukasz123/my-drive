# Base image for rapsberrypi 3 target
FROM rustembedded/cross:armv7-unknown-linux-gnueabihf

# Install pkg-config
RUN dpkg --add-architecture armhf && \
	    apt-get update && \
	    apt-get install --assume-yes  pkg-config
