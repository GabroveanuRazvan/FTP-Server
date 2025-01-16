ARG RUST_VERSION=1.82.0
ARG APP_NAME=server

################################################################################
# Create a stage for building the application.

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
RUN apk add --no-cache clang lld musl-dev git

# Copy the server and utils into the app folder
COPY ./server /app/server
COPY ./utils /app/utils

# Run the server project and copy the executable into /bin
WORKDIR /app/server
RUN cargo build --release && \
    cp ./target/release/$APP_NAME /bin/server

################################################################################
# Create a new stage for running the application.

FROM ubuntu:latest AS final

# Create a non-privileged user that the app will run under.
ARG UID=10001
RUN useradd \
            --system \
            --no-create-home \
            --home "/nonexistent" \
            --shell "/usr/sbin/nologin" \
            --uid "${UID}" \
            appuser

USER appuser

# Copy the related directories and files that the server depends on to run
#WORKDIR /server/data
WORKDIR /server

# Copy the executable
COPY --from=build /bin/server /server/ftp-server

EXPOSE 7878 50000 50001 50002 50003 50004 50005 50006 50007 50008 50009 50010 50011 50012 50013 50014 50015 50016 50017 50018 50019 50020 50021 50022 50023 50024 50025 50026 50027 50028 50029 50030 50031 50032 50033 50034 50035 50036 50037 50038 50039 50040 50041 50042 50043 50044 50045 50046 50047 50048 50049 50050 50051 50052 50053 50054 50055 50056 50057 50058 50059 50060 50061 50062 50063 50064 50065 50066 50067 50068 50069 50070 50071 50072 50073 50074 50075 50076 50077 50078 50079 50080 50081 50082 50083 50084 50085 50086 50087 50088 50089 50090 50091 50092 50093 50094 50095 50096 50097 50098 50099 50100


CMD ["/server/ftp-server"]
