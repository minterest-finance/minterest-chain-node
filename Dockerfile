FROM liuchong/rustup

ARG Mainteiner="Nick Lototskiy"
ARG Name="Minterest Platform"
ARG Version="0.0.1"

USER root
WORKDIR platform
COPY . .

RUN apt update && apt install -y llvm clang curl make
RUN make init && make build

EXPOSE 9944 9933

ENTRYPOINT ["make"]
CMD ["run"]