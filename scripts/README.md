# Parachain network setup

1. Get Polkadot node and place it as `../../polkadot/target/release/polkadot` relative to scripts folder
2. Install https://www.npmjs.com/package/@polkadot/api-cli
3. `cd scripts`
4. Run `./export_relay_spec.sh` and `./export_para.sh` to create relay spec, para wasm and para genesis
5. Start relay nodes with `./run_relays.sh` and crate new terminal for following commands
6. Register parachain with `./reg_para.sh`
7. Start para nodes with `./run_paras.sh`
8. Parachain should sync in relay blocks and after 2 minutes it should start to produce blocks.
9. Stop nodes with Ctrl-C