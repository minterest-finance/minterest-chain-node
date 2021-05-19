FROM ubuntu:20.10

ARG Mainteiner="Nick Lototskiy"
ARG Name="Minterest Platform"
ARG Version="0.0.1"

WORKDIR platform
COPY . .

RUN apt update && apt install -y llvm clang curl rustc make && curl https://sh.rustup.rs -sSf | sh
RUN make init
RUN make build

ENTRYPOINT ["make"]
CMD ["run"]