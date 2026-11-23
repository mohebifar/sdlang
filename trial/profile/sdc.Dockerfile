# The Linux build of `sdc` that the language arm runs inside profile trial-v1. It is built
# apart from the profile so that every arm runs in the same image; only the language arm's
# containers mount the binary. Debian bookworm on both sides, so the glibc matches.
#
# docker build -f trial/profile/sdc.Dockerfile --output trial/profile/bin cli
FROM rust:1.89-bookworm@sha256:948f9b08a66e7fe01b03a98ef1c7568292e07ec2e4fe90d88c07bb14563c84ff AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked --bin sdc

FROM scratch
COPY --from=build /src/target/release/sdc /sdc
