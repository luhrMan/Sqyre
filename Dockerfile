FROM ghcr.io/hybridgroup/opencv:4.11.0 AS builder

FROM golang:1.23.3-alpine
COPY --from=builder . /go/src/gocv.io/x/gocv/
ENV CGO_ENABLED=1
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download && go mod verify
RUN apk add --no-cache \
gcc \
g++ \
libc-dev \
linux-headers \
libpng \
tesseract-ocr \
tesseract-ocr-dev \
mesa-dev \
libx11-dev \
libxi-dev \
libxkbcommon-x11 \
libxkbcommon \
libxkbcommon-dev \
libxtst-dev \
libxcursor-dev \
libxrandr-dev \
libxinerama-dev \
xorg-server-dev \
libxxf86vm-dev
#
COPY . .
RUN go build -v -o bin .
#
# FROM ghcr.io/hybridgroup/opencv:4.11.0
#
# RUN apk add --no-cache \
# cmake \
#
# COPY . .
# RUN go build -v -o bin .
#
#
ENTRYPOINT ["/app/bin"]
#
CMD ["go", "run", "main.go"]