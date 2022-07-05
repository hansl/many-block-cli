use chrono::DateTime;
use clap::Parser;
use many::server::module::blockchain::{BlockArgs, BlockReturns, InfoReturns};
use many::server::ManyUrl;
use many::types::blockchain::SingleBlockQuery;
use many::types::identity::CoseKeyIdentity;
use many::Identity;
use std::time::SystemTime;
use tabled::Tabled;

#[derive(Parser)]
struct Opts {
    /// The server to list the blocks from.
    server: ManyUrl,

    /// Number of blocks to query (default to 30).
    #[clap(long, default_value = "30")]
    count: u64,

    /// Max Height. By default, this will be the last block (calling blockchain.info).
    #[clap(long)]
    max_height: Option<u64>,
}

#[derive(Tabled)]
struct BlockRow {
    #[tabled(rename = "Height")]
    height: u64,

    #[tabled(rename = "# Txs")]
    tx_count: u64,

    #[tabled(rename = "AppHash")]
    app_hash: String,

    #[tabled(rename = "Block Time (UTC)")]
    block_time: String,

    #[tabled(rename = "Δ T")]
    delta: String,
}

fn main() {
    let opts: Opts = Opts::parse();

    let client = many_client::ManyClient::new(
        opts.server,
        Identity::anonymous(),
        CoseKeyIdentity::anonymous(),
    )
    .expect("Could not create client.");

    let max_height = opts.max_height.unwrap_or_else(|| {
        let info = client.call("blockchain.info", ()).expect("Could not call.");
        let info: InfoReturns = minicbor::decode(&info.data.expect("Error from the server:"))
            .expect("Could not decode");

        info.latest_block.height
    });

    let min_height = if max_height > opts.count {
        max_height - opts.count
    } else {
        0
    };

    let mut last_block_time: Option<SystemTime> = None;
    let blocks = (min_height..=max_height)
        .into_iter()
        .map(|h| {
            client
                .call(
                    "blockchain.block",
                    BlockArgs {
                        query: SingleBlockQuery::Height(h),
                    },
                )
                .expect("Could not call to get block")
        })
        .map(|message| {
            minicbor::decode::<BlockReturns>(&message.data.expect("Error from the server:"))
                .expect("Invalid serialization")
                .block
        })
        .map(|block| {
            let datetime: DateTime<chrono::Utc> = block.timestamp.0.into();
            let delta = if let Some(lbt) = last_block_time {
                humantime::format_duration(block.timestamp.0.duration_since(lbt).unwrap())
                    .to_string()
            } else {
                "".to_string()
            };
            last_block_time = Some(block.timestamp.0);

            BlockRow {
                height: block.id.height,
                tx_count: block.txs_count,
                app_hash: block
                    .app_hash
                    .map_or("-".to_string(), |h| hex::encode(&h).to_string()),
                block_time: datetime.format("%F %T").to_string(),
                delta,
            }
        })
        .collect::<Vec<_>>();

    println!(
        "{}",
        tabled::Table::new(blocks).with(tabled::Style::github_markdown())
    );
}
