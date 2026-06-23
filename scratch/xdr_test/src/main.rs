use stellar_xdr::curr::TransactionEnvelope;

fn main() {
    // We match on TransactionEnvelope to verify variants and fields.
    // This is to make sure we can extract the max bid fee correctly.
    let env = TransactionEnvelope::Tx(todo!());
    match env {
        TransactionEnvelope::Tx(ref envelope) => {
            let _ = envelope.tx.fee;
        }
        TransactionEnvelope::TxFeeBump(ref envelope) => {
            let _ = envelope.tx.fee;
        }
        TransactionEnvelope::TxV0(ref envelope) => {
            let _ = envelope.tx.fee;
        }
    }
}
