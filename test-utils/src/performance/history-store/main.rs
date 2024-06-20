use std::{fs, path::PathBuf, process::exit, time::Instant};

use clap::Parser;
use nimiq_blockchain::{
    interface::HistoryInterface, light_history_store::LightHistoryStore, HistoryStore,
};
use nimiq_database::{
    mdbx::MdbxDatabase,
    traits::{Database, WriteTransaction},
    DatabaseProxy,
};
use nimiq_genesis::NetworkId;
use nimiq_keys::Address;
use nimiq_primitives::{coin::Coin, policy::Policy};
use nimiq_transaction::{
    historic_transaction::{HistoricTransaction, HistoricTransactionData},
    ExecutedTransaction, Transaction as BlockchainTransaction,
};
use rand::{thread_rng, Rng};
use tempfile::tempdir;

/// Command line arguments for the binary
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of batches to add
    #[arg(short, long)]
    batches: u32,
    /// Transactions per block
    #[arg(short, long)]
    tpb: u32,
    /// Use the light history store
    #[arg(short, long, action = clap::ArgAction::Count)]
    light: u8,
    /// Rounds (Batches operations)
    #[arg(short, long)]
    rounds: Option<u32>,
    /// Loops: repeat the whole operation multiple times.
    #[arg(short, long)]
    loops: Option<u32>,
}

fn create_transaction(block: u32, value: u64) -> HistoricTransaction {
    let mut rng = thread_rng();
    HistoricTransaction {
        network_id: NetworkId::UnitAlbatross,
        block_number: block,
        block_time: 0,
        data: HistoricTransactionData::Basic(ExecutedTransaction::Ok(
            BlockchainTransaction::new_basic(
                Address(rng.gen()),
                Address(rng.gen()),
                Coin::from_u64_unchecked(value),
                Coin::from_u64_unchecked(0),
                0,
                NetworkId::Dummy,
            ),
        )),
    }
}

// Creates num_txns for the given block number, using consecutive values
fn gen_hist_txs_block(block_number: u32, num_txns: u32) -> Vec<HistoricTransaction> {
    let mut txns = Vec::new();

    for i in 0..num_txns {
        txns.push(create_transaction(block_number, i as u64));
    }
    txns
}

fn history_store_populate(
    history_store: &Box<dyn HistoryInterface + Sync + Send>,
    env: &DatabaseProxy,
    tpb: u32,
    batches: u32,
    rounds: u32,
    loops: u32,
    db_file: PathBuf,
) {
    let num_txns = tpb;
    let loops = loops - 1;

    let number_of_blocks = Policy::blocks_per_batch() * batches;
    let loop_ofset = loops * number_of_blocks;
    let mut txns_per_block = Vec::new();

    println!(" Generating txns.. ");
    for block_number in 0..number_of_blocks {
        txns_per_block.push(gen_hist_txs_block(loop_ofset + 1 + block_number, num_txns));
    }
    println!(" Done generating txns. ");

    println!(" Adding txns to history store");
    let start = Instant::now();

    let batches_per_round = batches / rounds;
    let blocks_per_round = number_of_blocks / rounds;
    println!(
        " Number of rounds: {}, batches per round {} blocks per round {}",
        rounds, batches_per_round, blocks_per_round
    );

    for round in 0..rounds {
        let round_start = Instant::now();
        let mut txn = env.write_transaction();

        for block_number in 0..blocks_per_round {
            history_store.add_to_history(
                &mut txn,
                (loop_ofset) + (round * blocks_per_round) + (1 + block_number),
                &txns_per_block[((round * blocks_per_round) + block_number) as usize],
            );
        }
        let commit_start = Instant::now();
        txn.commit();
        let round_duration = round_start.elapsed();
        let commit_duration = commit_start.elapsed();

        println!(
            "...{:.2}s to process round {}, {:.2}s for commit",
            round_duration.as_millis() as f64 / 1000_f64,
            1 + round,
            commit_duration.as_millis() as f64 / 1000_f64,
        );
    }

    let duration = start.elapsed();

    let db_file_size = fs::metadata(db_file.to_str().unwrap()).unwrap().len();

    println!(
        " {:.2}s to add {} batches, {} tpb, DB size: {:.2}Mb, rounds: {}, total_txns: {}",
        duration.as_millis() as f64 / 1000_f64,
        batches,
        num_txns,
        db_file_size as f64 / 1000000_f64,
        rounds,
        num_txns * number_of_blocks
    );
}

fn history_store_prune(
    history_store: &Box<dyn HistoryInterface + Sync + Send>,
    env: &DatabaseProxy,
) {
    let mut txn = env.write_transaction();
    println!("Pruning the history store..");
    let start = Instant::now();

    history_store.remove_history(&mut txn, Policy::epoch_at(1));

    let duration = start.elapsed();

    println!(
        "It took: {:.2}s, to prune the history store",
        duration.as_millis() as f64 / 1000_f64,
    );
}

fn main() {
    let args = Args::parse();

    let policy_config = Policy::default();

    let _ = Policy::get_or_init(policy_config);

    let temp_dir = tempdir().expect("Could not create temporal directory");
    let tmp_dir = temp_dir.path().to_str().unwrap();
    let db_file = temp_dir.path().join("mdbx.dat");
    log::debug!("Creating a non volatile environment in {}", tmp_dir);
    let env = MdbxDatabase::new(tmp_dir, 1024 * 1024 * 1024 * 1024, 21).unwrap();

    let history_store = if args.light > 0 {
        println!("Exercising the light history store");
        Box::new(LightHistoryStore::new(
            env.clone(),
            NetworkId::UnitAlbatross,
        )) as Box<dyn HistoryInterface + Sync + Send>
    } else {
        println!("Exercising the history store");
        Box::new(HistoryStore::new(env.clone(), NetworkId::UnitAlbatross))
            as Box<dyn HistoryInterface + Sync + Send>
    };

    let rounds = args.rounds.unwrap_or(1);
    let loops = args.loops.unwrap_or(1);

    for loop_number in 1..(1 + loops) {
        println!("Current loop {}", loop_number);

        history_store_populate(
            &history_store,
            &env,
            args.tpb,
            args.batches,
            rounds,
            loop_number,
            db_file.clone(),
        );
    }

    history_store_prune(&history_store, &env);

    let _ = fs::remove_dir_all(temp_dir);

    exit(0);
}
