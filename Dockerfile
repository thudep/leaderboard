FROM rust:alpine AS build
COPY . /app
WORKDIR /app
RUN apk upgrade --no-cache && apk --no-cache add musl-dev && cargo build --release

FROM alpine AS runtime
COPY --from=build /app/target/release/leaderboard /bin
RUN mkdir -p /etc/leaderboard
COPY leaderboard.toml /etc/leaderboard/
USER root
EXPOSE 80
CMD ["leaderboard"]
