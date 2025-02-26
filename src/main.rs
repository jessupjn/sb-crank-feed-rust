use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::message::v0::Message;
use solana_sdk::message::VersionedMessage::V0;
use solana_sdk::signer::keypair::read_keypair_file;
use solana_sdk::transaction::VersionedTransaction;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::{error::Error, str::FromStr, sync::LazyLock};
use switchboard_on_demand_client::*;

/// Devnet RPC URL.
static RPC_URL: LazyLock<String> = LazyLock::new(|| "https://api.devnet.solana.com".to_string());

/// Switchboard Devnet Default Queue.
static QUEUE: LazyLock<Pubkey> =
    LazyLock::new(|| Pubkey::from_str("EYiAmGSdsQTuCw413V5BzaruWuCCSDgTPtBGvLkXHbe7").unwrap());

/// The feed to crank.
static FEED: LazyLock<Pubkey> =
    LazyLock::new(|| Pubkey::from_str("FwzcymbxHJ7CArSmAwnguzyEBakLq2h3TZjHsz1r51rr").unwrap());

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Payer keypair path
    #[arg(short, long, default_value = "~/.config/solana/id.json")]
    keypair: String,
}

async fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let sb_context = SbContext::new();
    let client = RpcClient::new_with_commitment(
        RPC_URL.to_string(),
        CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        },
    );
    println!("\nUsing RPC URL: {}", client.url());

    println!("\nLoading payer keypair at {:?}", args.keypair);
    let payer = read_keypair_file(args.keypair).unwrap();
    println!("Payer loaded ({:?})", payer.pubkey());

    let queue = QueueAccountData::load(&client, &QUEUE).await.unwrap();
    let gateways = &queue.fetch_gateways(&client).await.unwrap();
    println!("\nLoaded {:?} gateways", gateways.len());

    let crossbar = CrossbarClient::default();

    println!("\nFetching update instruction from gateway...");
    let (ix, responses, num_success, luts) = PullFeed::fetch_update_ix(
        sb_context,
        &client,
        FetchUpdateParams {
            feed: *FEED,
            payer: payer.pubkey(),
            gateway: gateways[0].clone(),
            crossbar: Some(crossbar),
            num_signatures: Some(6),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    println!("Fetched {:?} successful responses...", num_success);
    responses.iter().enumerate().for_each(|(idx, resp)| {
        println!(
            "  {:?}: {:?} from {:?}",
            idx,
            resp.value.map_or("ERR".to_string(), |v| v.to_string()),
            resp.oracle.to_string()
        );
    });

    println!("\nBuilding transaction...");
    let blockhash = client.get_latest_blockhash().await.unwrap();
    let msg = Message::try_compile(
        &payer.pubkey(),
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            ComputeBudgetInstruction::set_compute_unit_price(69_000),
            ix.clone(),
        ],
        &luts,
        blockhash,
    )
    .unwrap();

    let versioned_tx = VersionedTransaction::try_new(V0(msg), &[&payer]).unwrap();
    let result = client.simulate_transaction(&versioned_tx).await.unwrap();
    println!(
        "Simulation logs: {:#?}",
        result.value.logs.unwrap_or(vec![])
    );

    // If simulation fails, stop script here.
    if let Some(err) = result.value.err {
        println!("\nSimulation failed: {:#?}", err);
        return Err(format!("Simulation failed: {:#?}", err).into());
    }

    // If simulation succeeds, send transaction.
    println!("\nSending transaction...");
    let sig = client
        .send_and_confirm_transaction(&versioned_tx)
        .await
        .unwrap();
    println!("Transaction sent and confirmed: {:#?}", sig);
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(e) = run(args).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
