# Build container
FROM alpine:3.12 as builder
COPY install.sh /install.sh
ARG CORVUS_VERSION="v0.0.0"
RUN /install.sh ${CORVUS_VERSION}

# # Target container
FROM alpine:3.12
COPY --from=builder /dist/corvus /bin/corvus
COPY ./rootfs/ /

ENTRYPOINT [ "/bin/corvus" ]
CMD [ "-c", "/etc/corvus/corvus.toml" ]
