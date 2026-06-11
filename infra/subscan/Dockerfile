FROM golang:1.26.2 AS builder

WORKDIR /subscan

RUN git clone --depth 1 https://github.com/subscan-explorer/subscan-essentials.git .
RUN go mod download

WORKDIR /subscan/cmd
RUN go build -o subscan

FROM alpine:3

WORKDIR /subscan
COPY --from=builder /subscan/configs configs
COPY --from=builder /subscan/cmd/subscan cmd/subscan

COPY configs/config.yaml configs/config.yaml
COPY configs/source/artemis.json configs/source/artemis.json

WORKDIR /subscan/cmd
RUN apk update && apk add gcompat
ENTRYPOINT ["/subscan/cmd/subscan"]
EXPOSE 4399
