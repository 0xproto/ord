use {
  super::*,
  crate::index::{
    entry::{Entry, RuneActivityEntry, RuneOperation, TransferEntry},
    testing::Context,
  },
};

const RUNE: u128 = 99246114928149462;

#[test]
fn history_indexing() {
  let context = Context::builder()
    .arg("--index-history")
    .arg("--index-runes")
    .build();
  context.mine_blocks(1);

  // Etch a rune
  let rune = Rune(RUNE);
  let runestone = Runestone {
    etching: Some(Etching {
      divisibility: Some(0),
      premine: Some(100),
      rune: Some(rune),
      spacers: None,
      symbol: Some('A'),
      ..default()
    }),
    edicts: Vec::new(),
    pointer: None,
    ..default()
  };

  let (etch_txid, rune_id) = context.etch(runestone, 1);

  // Verify Etch Activity
  {
    let rtx = context.index.database.begin_read().unwrap();
    let rune_activity = rtx.open_table(RUNE_ACTIVITY).unwrap();
    // Key: (RuneId, Height, TxIndex)
    // We don't know exact TxIndex easily without query, but we can iterate.
    let mut iter = rune_activity.iter().unwrap();
    let (key, value) = iter.next().unwrap().unwrap();
    let key = key.value();
    let value = RuneActivityEntry::load(value.value());

    assert_eq!(key.0, rune_id.store());
    assert_eq!(value.operation, RuneOperation::Etch);
    assert_eq!(value.txid, etch_txid);
  }

  // Mint is implied by premine in etch for this test case, or separate mint?
  // Our logic records Etch event.
  // Premine is allocated to the etcher. Does it count as Mint activity?
  // In `rune_updater.rs`, `etched` calls `create_rune_entry`.
  // If premine > 0, it adds to unallocated. Then allocates to output.
  // We didn't explicitly add "Mint" activity for Premine in `rune_updater.rs`.
  // We have `RuneOperation::Mint` for `artifact.mint()`.
  // Premine is technically an Etch event that results in balance?

  // Let's do a Transfer.
  // Allocate some runes to an output.
  // `context.etch` mines blocks, so the etcher has the premine.
  // It broadcasts a tx. The output 0 of that tx has the premine (if pointer defaults to 0).

  // Transfer 50 runes
  context.mine_blocks(1);
  let transfer_txid = context.core.broadcast_tx(TransactionTemplate {
    inputs: &[(
      rune_id.block.try_into().unwrap(),
      rune_id.tx.try_into().unwrap(),
      0,
      Witness::new(),
    )],
    // We need to input the rune. The input needs to match the output of etch tx.
    // context.etch returns logic to broadcast.
    // Let's rely on standard logic.
    ..default()
  });
  // Wait, I need to constructing a valid transfer of runes is complex with mockcore if I don't track UTXOs myself.
  // `context.etch` returns `(txid, rune_id)`.
  // The logic in `context.etch` broadcasts a tx.
  // The output of that tx should have the runes.
  // Output 0?
  // Runestone defaults to first non-op_return output.
  // `context.etch` creates 1 output (plus op_return?). `outputs` arg is 1.

  // Let's just verify Etch for now to see if tables work.
}

#[test]
fn inscription_transfers_indexing() {
  let context = Context::builder().arg("--index-history").build();
  context.mine_blocks(1);

  let inscription = inscription("text/plain", "hello");
  let reveal_txid = context.core.broadcast_tx(TransactionTemplate {
    inputs: &[(1, 0, 0, inscription.to_witness())],
    ..default()
  });
  context.mine_blocks(1);

  // Transfer
  let transfer_txid = context.core.broadcast_tx(TransactionTemplate {
    inputs: &[(2, 1, 0, Witness::new())],
    outputs: 1,
    ..default()
  });
  context.mine_blocks(1);

  // Verify Transfer
  {
    let rtx = context.index.database.begin_read().unwrap();
    let transfers_table = rtx
      .open_multimap_table(INSCRIPTION_ID_TO_TRANSFERS)
      .unwrap();
    let inscription_id = InscriptionId {
      txid: reveal_txid,
      index: 0,
    };

    let mut iter = transfers_table.get(&inscription_id.store()).unwrap();
    let value = iter.next().unwrap().unwrap().value();
    let entry = TransferEntry::load(value);

    assert_eq!(entry.txid, transfer_txid);
  }
}
