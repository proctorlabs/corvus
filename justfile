# platforms := "linux/arm64,linux/amd64,linux/386,linux/arm/v7,linux/arm/v6"
platforms := "linux/arm64,linux/amd64,linux/386"
docker_binary := "0.1.0-alpha7"

fmt:
    #!/usr/bin/env bash
    set -Eeuo pipefail
    cargo +nightly fmt

run: fmt
    #!/usr/bin/env bash
    set -Eeuo pipefail
    cargo build
    sudo ./target/debug/corvus -v

watch:
    #!/usr/bin/env bash
    set -Eeuo pipefail
    watchexec -w src just run

run-docker:
    #!/usr/bin/env bash
    set -Eeuo pipefail

    docker run -it --rm \
        -v $PWD/corvus.toml:/corvus.toml \
        --cap-add=NET_ADMIN \
        --net=host \
        $(docker build -q .) \
            corvus -v

docker-xbuild-enable:
    #!/usr/bin/env bash
    set -Eeuo pipefail

    docker run --rm --privileged multiarch/qemu-user-static --reset -p yes

docker-xbuild-setup:
    #!/usr/bin/env bash
    set -Eeuo pipefail

    docker buildx create --platform {{platforms}} --name cross-builder --append
    docker buildx use cross-builder

docker-xbuild-build:
    #!/usr/bin/env bash
    set -Eeuo pipefail

    docker buildx build --platform {{platforms}} \
        -t corvus -f docker/Dockerfile \
        --build-arg CORVUS_VERSION="{{docker_binary}}" docker/

docker-xbuild-run arch:
    #!/usr/bin/env bash
    set -Eeuo pipefail

    docker buildx build --platform linux/{{arch}} \
        --load -t corvus -f docker/Dockerfile \
        --build-arg CORVUS_VERSION="{{docker_binary}}" docker/

    docker run --rm -it \
        -v $PWD/corvus.toml:/etc/corvus/corvus.toml \
        --cap-add=NET_ADMIN \
        --net=host \
            corvus
