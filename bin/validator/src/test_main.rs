use alloy_primitives::{BlockHash, BlockNumber};
use alloy_provider::RootProvider;
use alloy_rpc_types_eth::Block;
use alloy_transport_http::{Client, Http};
use eyre::{Result, anyhow};
use megaeth_salt::{BlockWitness, EphemeralSaltState, StateRoot};
use megaeth_salt_stateless::{
    SaltWitnessState,
    generator::get_witness_state,
    validator::{
        PlainKeyUpdate, WitnessProvider,
        evm::replay_block,
        file::{load_contracts_file, load_json_file, read_block_hash_by_number_from_file},
    },
};
use revm::{
    db::in_memory_db::CacheDB,
    primitives::{B256, Bytecode},
};
use std::{collections::HashMap, path::PathBuf, time::Instant};
use tokio::runtime::Handle;

#[tokio::main]
async fn main() -> Result<()> {
    let stateless_dir = PathBuf::from("./bin/stateless/validator/test_data/stateless");
    let block_counter = 9;

    test_scan_and_validate_block_witnesses(&stateless_dir, block_counter)?;
    Ok(())
}

/// 验证区块执行结果
fn test_scan_and_validate_block_witnesses(
    stateless_dir: &PathBuf,
    mut block_counter: u64,
) -> Result<()> {
    while block_counter < 19 {
        println!("block_number: {}", block_counter);
        let start = Instant::now();
        // 从 witness 文件名读取区块 hash。比如从以下读取第8个区块的 hash
        // 8.0xb3bda63a35f00b666dc7dcb3542ebd4d2755ecbbb97d5b5b312b57b5124658fc.w
        let pre_block_hash =
            read_block_hash_by_number_from_file(block_counter - 1, &stateless_dir)?;
        let block_hash = read_block_hash_by_number_from_file(block_counter, &stateless_dir)?;

        let block_hash = block_hash[0];
        let pre_block_hash = pre_block_hash[0];
        // 获取 generator client 生成的 WitnessStatus
        let witness_status = get_witness_state(&stateless_dir, &(block_counter, block_hash))?;

        // 如果 WitnessStatus 状态为 Idle 或 Processing，则说明 generator client 未完成生成 witness
        if witness_status.status == SaltWitnessState::Idle
            || witness_status.status == SaltWitnessState::Processing
        {
            println!("invalid witness_status: {:?}", witness_status);
            continue;
        }

        // 反序列化 witness 数据，获取 BlockWitness 结构
        let (block_witness, _witness_size): (BlockWitness, usize) =
            bincode::serde::decode_from_slice(
                &witness_status.witness_data,
                bincode::config::legacy(),
            )
            .map_err(|e| anyhow!("Failed to parse witness: {}", e))?;

        let block_path = stateless_dir.join("witness");

        // 从文件中读取区块数据，block_a 是前一个区块，block_b 是当前区块(要验证的区块)
        let block_a: Block = load_json_file(
            &block_path,
            &block_file_name(block_counter - 1, pre_block_hash),
        )?;
        let block_b: Block =
            load_json_file(&block_path, &block_file_name(block_counter, block_hash))?;
        println!(
            "block {}'s tx len: {}",
            block_counter,
            block_b.transactions.len()
        );

        // 从文件中读取合约的 bytecode。
        let contracts: HashMap<B256, Bytecode> =
            load_contracts_file(&block_path, &contracts_file_name())?;

        let old_state_root = block_a.header.state_root;
        let new_state_root = block_b.header.state_root;

        // 验证 witness 数据是否正确
        block_witness.verify_proof::<BlockWitness, BlockWitness>(old_state_root)?;

        // 创建 WitnessProvider 结构，用于执行 evm。
        // 具体参考 crates/salt/stateless/src/validator.rs
        let rt = Handle::current();
        let provider =
            RootProvider::<Http<Client>>::new_http("http://localhost:9545".parse().unwrap());
        let witness_provider = WitnessProvider {
            witness: block_witness.clone(),
            contracts,
            provider,
            rt,
        };

        let mut db = CacheDB::new(witness_provider);

        // 执行区块，状态的变化会写入 db 里面的 cache 部分，即 witness_provider 不会变化。
        // 具体参考 crates/salt/stateless/src/validator/evm.rs
        replay_block(block_b.clone(), &mut db)?;

        // 将状态变化转换为 PlainKey PlainValue 的结果，
        // 具体参考 crates/salt/stateless/src/validator.rs
        let plain_state: PlainKeyUpdate = db.accounts.into();

        // 更新 salt 状态，获取 root
        let state_updates = EphemeralSaltState::new(&block_witness)
            .update(&plain_state.data)
            .unwrap();

        let mut trie = StateRoot::new();
        let (new_trie_root, _trie_updates) = trie.update(&block_witness, &state_updates).unwrap();
        if new_trie_root != new_state_root {
            println!(
                "failed,new_trie_root: {:?}, new_state_root: {:?}",
                new_trie_root, new_state_root
            );
        } else {
            println!(
                "success,new_trie_root: {:?}, new_state_root: {:?}",
                new_trie_root, new_state_root
            );
        }

        block_counter += 1;
        println!("total cost {:?}", start.elapsed());
    }

    Ok(())
}

/// return the contracts file name to the given block number and block hash
fn contracts_file_name() -> String {
    "contracts.txt".to_string()
}

fn block_file_name(block_num: BlockNumber, block_hash: BlockHash) -> String {
    format!("{}.{}.block.json", block_num, block_hash)
}
