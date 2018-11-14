# thanks https://whitfin.io/speeding-up-rust-docker-builds/

# select build image
FROM rust as build

# create a new empty shell project
RUN USER=root cargo new --bin bottle
WORKDIR /bottle

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src
# copy migrations
COPY ./migrations ./migrations

# build for release
RUN rm ./target/release/deps/bottle*
RUN cargo build --release

# our final base
FROM busybox

# copy the build artifact from the build stage
COPY --from=build /bottle/target/release/bottle .

# set the startup command to run your binary
CMD ["./bottle"]