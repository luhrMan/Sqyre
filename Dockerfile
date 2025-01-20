FROM gocv/opencv:4.11.0 AS builder
COPY . .

FROM golang:1.23.3-alpine
ENV CGO_ENABLED=1
ENV PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:/usr/lib/pkgconfig
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download && go mod verify
RUN go get -u gocv.io/x/gocv
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

COPY . /app
# RUN go build -v -o bin .

# ENTRYPOINT ["/app/bin"]
CMD ["cat", "/usr/local/lib/pkgconfig/opencv4.pc"]
# CMD ["go", "run", "main.go"]