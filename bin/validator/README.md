# Validator

## multi validator
run validators

First select a port for the server to listen on and make sure the port is not occupied. Here we use 3000:

terminal 1:
```sh
cd mega-reth/bin/stateless/validator


cargo run --release --bin megaeth-validator -- --datadir /Users/worker13/workspace/data/generatordb --api http://127.0.0.1:9545 --port 3000
```

```sh
curl -X POST http://localhost:3000/ -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "stateless_getValidation", "params": ["2.0xb041ff7664a260be57617a2c4f1737c42098e2256b11f19e7317d397883a2038", "3.0xb7ea22fc6fa1f85dddc7b43e9dbafbf20381c58bf0e5968572beaacbb3308605", "11.0x882e55f7e9e04c531cad086175ec4baa17e28b767974b4613f466b545c06f03d"], "id": 1}'
```

The second one does not need to start the server. If the port information is not passed, the server will not be started.
terminal 2:
```sh
cd mega-reth/bin/stateless/validator


cargo run --release --bin megaeth-validator -- --datadir /Users/worker13/workspace/data/generatordb --api http://127.0.0.1:9545
```

## test case
run test:
```sh
cd mega-reth

cargo run --release -p megaeth-validator --bin test_main
```
